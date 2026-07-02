//! Réponses tableau de bord : rapides d'abord, Internet, puis LLM léger.

use uuid::Uuid;

use crate::ai::agent::{ChatColsRequest, ChatDisplayBlock, ChatReply, EntityCreateAction, RegistryEntityCreateAction};
use crate::ai::intent_filters::wants_internet_research_intent;
use crate::entity::create_draft;
use crate::entity::list_preview::{self, visible_chat_message};
use crate::entity::registry;
use crate::entity::registry_create_draft;
use crate::privileges::{can_create_registry_entity, has_privilege};
use crate::ai::llama_server::{ChatMessage, LlamaServer};
use crate::ai::quick_answers::try_quick_answer;
use crate::ai::translate::{core_message, finalize_with_translation, try_translate_previous_reply};
use crate::ai::web_search;
use crate::db::Database;
use crate::session::SessionUser;

/// Répond à une question pratique depuis l'accueil (sans pipeline agent complet).
pub fn answer_practical(
    db: &Database,
    user: &SessionUser,
    conversation_id: Option<&str>,
    user_message: &str,
) -> Result<ChatReply, String> {
    let conv_id = conversation_id
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    if conversation_id.is_none() {
        let title: String = user_message.chars().take(48).collect();
        db.ai_create_conversation(&conv_id, &user.id, &title)
            .map_err(|e| e.to_string())?;
    }

    db.ai_add_message(&conv_id, "user", user_message)
        .map_err(|e| e.to_string())?;

    if let Some(result) = try_translate_previous_reply(db, &conv_id, user_message) {
        return match result {
            Ok(msg) => store_assistant_raw(db, &conv_id, &msg, vec![], None, None, vec![], None),
            Err(e) => store_assistant_raw(db, &conv_id, &e, vec![], None, None, vec![], None),
        };
    }

    let core = core_message(user_message);

    if let Some(msg) = try_quick_answer(&core) {
        return store_assistant(db, &conv_id, user_message, &msg, vec![], None, vec![]);
    }

    if let Ok(reg) = registry::load(&db.data_dir) {
        if let Some(reg_draft) = registry_create_draft::match_registry_create_draft(user_message, &reg)
        {
            let allowed = can_create_registry_entity(&user.privileges);
            let msg = if allowed {
                reg_draft.assistant_message.clone()
            } else {
                "Tu n'as pas le privilège pour créer une entité dans le registre (parametres:entites:creer). Contacte un administrateur.".into()
            };
            let open_registry = if allowed {
                Some(RegistryEntityCreateAction {
                    initial_entity: serde_json::to_value(&reg_draft.initial_entity)
                        .unwrap_or(serde_json::Value::Null),
                })
            } else {
                None
            };
            return store_assistant_registry(
                db,
                &conv_id,
                user_message,
                &msg,
                open_registry,
            );
        }

        if let Some(draft) = create_draft::match_create_draft(user_message, &reg) {
            let priv_key = format!("{}:creer", draft.entity_key);
            if has_privilege(&user.privileges, &priv_key) {
                let action = EntityCreateAction {
                    entity_key: draft.entity_key.clone(),
                    initial_data: draft.initial_data.clone(),
                };
                return store_assistant(
                    db,
                    &conv_id,
                    user_message,
                    &draft.assistant_message,
                    vec![],
                    Some(action),
                    vec![],
                );
            }
        }
    }

    if let Ok(Some(result)) = list_preview::try_list_preview(db, user, &conv_id, user_message)
    {
        return store_assistant_raw(
            db,
            &conv_id,
            &result.raw_message,
            vec![],
            None,
            None,
            result.display_blocks,
            result.cols_request,
        );
    }

    if web_search::is_enabled(&db.data_dir) && wants_internet_research_intent(&core) {
        let history: Vec<(String, String)> = db
            .ai_list_messages(&conv_id, 8)
            .unwrap_or_default()
            .into_iter()
            .map(|m| (m.role, m.content))
            .collect();
        let query = match web_search::build_contextual_query(&history, &core) {
            Some(q) => q,
            None => {
                let msg = "Que veux-tu que je recherche sur Internet ? Précise un sujet (par exemple « cherche Tayc sur internet »).";
                return store_assistant(db, &conv_id, user_message, msg, vec![], None, vec![]);
            }
        };
        match web_search::search(&db.data_dir, &query) {
            Ok(result) => {
                let msg = web_search::synthesize_answer(db, &core, &result)
                    .unwrap_or_else(|_| web_search::format_results_message(&result));
                return store_assistant(db, &conv_id, user_message, &msg, vec![], None, vec![]);
            }
            Err(e) => {
                let err_msg = format!(
                    "Je n'ai pas pu effectuer la recherche Internet ({e}). Vérifiez votre connexion ou réessayez."
                );
                return store_assistant(db, &conv_id, user_message, &err_msg, vec![], None, vec![]);
            }
        }
    }

    let msg = lightweight_llm_reply(db, &conv_id, &core)?;
    store_assistant(db, &conv_id, user_message, &msg, vec![], None, vec![])
}

fn store_assistant(
    db: &Database,
    conv_id: &str,
    user_message_full: &str,
    msg: &str,
    tool_results: Vec<crate::ai::tools::ToolResult>,
    open_entity_create: Option<EntityCreateAction>,
    display_blocks: Vec<ChatDisplayBlock>,
) -> Result<ChatReply, String> {
    let final_msg = finalize_with_translation(user_message_full, msg)?;
    store_assistant_raw(
        db,
        conv_id,
        &final_msg,
        tool_results,
        open_entity_create,
        None,
        display_blocks,
        None,
    )
}

fn store_assistant_registry(
    db: &Database,
    conv_id: &str,
    user_message_full: &str,
    msg: &str,
    open_registry_entity_create: Option<RegistryEntityCreateAction>,
) -> Result<ChatReply, String> {
    let final_msg = finalize_with_translation(user_message_full, msg)?;
    store_assistant_raw(
        db,
        conv_id,
        &final_msg,
        vec![],
        None,
        open_registry_entity_create,
        vec![],
        None,
    )
}

fn store_assistant_raw(
    db: &Database,
    conv_id: &str,
    msg: &str,
    tool_results: Vec<crate::ai::tools::ToolResult>,
    open_entity_create: Option<EntityCreateAction>,
    open_registry_entity_create: Option<RegistryEntityCreateAction>,
    display_blocks: Vec<ChatDisplayBlock>,
    cols_request: Option<ChatColsRequest>,
) -> Result<ChatReply, String> {
    db.ai_add_message(conv_id, "assistant", msg)
        .map_err(|e| e.to_string())?;
    let (visible, blocks) = if display_blocks.is_empty() {
        visible_chat_message(msg)
    } else {
        (visible_chat_message(msg).0, display_blocks)
    };
    Ok(ChatReply {
        conversation_id: conv_id.to_string(),
        message: visible,
        tool_results,
        display_blocks: blocks,
        cols_request,
        open_entity_create,
        open_registry_entity_create,
    })
}

fn lightweight_llm_reply(db: &Database, conv_id: &str, _core_message: &str) -> Result<String, String> {
    if !LlamaServer::model_ready() {
        return Err(
            "Modèle IA local absent. Installez le fichier GGUF (Paramètres > Assistant).".into(),
        );
    }
    let _ = LlamaServer::prepare(db, false);
    let app_name = crate::entity::branding::ecosystem_name(&db.data_dir);
    let web_on = web_search::is_enabled(&db.data_dir);
    let web_hint = if web_on {
        "Si tu ne sais pas, dis-le et propose une recherche Internet ou les écrans du tableau de bord."
    } else {
        "Tu n'as pas accès à Internet."
    };

    let mut messages = vec![ChatMessage {
        role: "system".into(),
        content: format!(
            "Tu es Loggy, assistant de {app_name}. Réponds en français, première personne (je), \
             courte et utile (2 à 6 phrases max). {web_hint} Pas de JSON ni LaTeX."
        ),
    }];
    if let Ok(history) = db.ai_list_messages(conv_id, 8) {
        for m in history {
            if m.role == "system" {
                continue;
            }
            messages.push(ChatMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            });
        }
    }

    let reply = LlamaServer::chat_with_options(Some(db), messages, 0.4, 280)?;
    Ok(reply.trim().to_string())
}
