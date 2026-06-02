use uuid::Uuid;

use crate::ai::config::project_root;
use crate::ai::intent_filters::{
    extract_web_search_query, infer_export_tool, infer_list_tool, infer_period_from_message,
    infer_statut_param, wants_action_intent, wants_detail_intent, wants_export_intent,
    wants_generate_loyers_intent, wants_internet_research_intent, wants_last_bien,
    wants_list_intent, wants_pay_finance_intent, wants_search_intent,
};
use crate::ai::quick_answers::try_quick_answer;
use crate::ai::translate::{core_message, finalize_with_translation, try_translate_previous_reply};
use crate::ai::web_search;
use crate::privileges::has_privilege;
use crate::ai::experience::{format_experience_hints, record_success, try_experience_intent};
use crate::ai::greetings::{classify_greeting, format_greeting_reply};
use crate::ai::fts_rewrite::rewrite_user_query_for_fts;
use crate::ai::llama_server::{ChatMessage, LlamaServer};
use crate::ai::rag::{build_project_knowledge, RagStore};
use crate::ai::tools::{
    execute_read_tool, format_tool_reply, is_write_tool, parse_tool_call,
    parse_create_from_user_message, parse_crud_from_user_message,
    parse_delete_all_from_user_message, parse_delete_from_user_message,
    parse_update_from_user_message, queue_write_action, ToolCall,
    ToolResult,
};
use crate::db::Database;
use crate::session::SessionUser;

#[derive(serde::Serialize, Clone)]
pub struct EntityCreateAction {
    pub entity_key: String,
    pub initial_data: serde_json::Value,
}

#[derive(serde::Serialize)]
pub struct ChatReply {
    pub conversation_id: String,
    pub message: String,
    pub tool_results: Vec<ToolResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub open_entity_create: Option<EntityCreateAction>,
}

pub struct Agent<'a> {
    db: &'a Database,
}

impl<'a> Agent<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    fn store_assistant_reply(
        &self,
        conv_id: &str,
        user_message_full: &str,
        msg: String,
        tool_results: Vec<ToolResult>,
    ) -> Result<ChatReply, String> {
        let final_msg = finalize_with_translation(user_message_full, &msg)?;
        self.db
            .ai_add_message(conv_id, "assistant", &final_msg)
            .map_err(|e| e.to_string())?;
        Ok(ChatReply {
            conversation_id: conv_id.to_string(),
            message: final_msg,
            tool_results,
            open_entity_create: None,
        })
    }

    pub fn reindex(&self) -> Result<usize, String> {
        let rag = RagStore::new(self.db);
        rag.clear().map_err(|e| e.to_string())?;
        let root = project_root();
        let mut total = 0usize;
        for (source, text) in build_project_knowledge(&root) {
            total += rag.index_text(&source, &text).map_err(|e| e.to_string())?;
        }
        for sub in ["knowledge", "validations"] {
            let dda_dir = self.db.data_dir.join("dda").join(sub);
            if !dda_dir.is_dir() {
                continue;
            }
            if let Ok(entries) = std::fs::read_dir(&dda_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("txt") {
                        if let Ok(text) = std::fs::read_to_string(&path) {
                            let source = path
                                .file_stem()
                                .and_then(|n| n.to_str())
                                .unwrap_or("dda");
                            total += rag
                                .index_text(&format!("dda_{sub}_{source}"), &text)
                                .map_err(|e| e.to_string())?;
                        }
                    }
                }
            }
        }
        let live = self.build_live_summary()?;
        total += rag
            .index_text("donnees_live", &live)
            .map_err(|e| e.to_string())?;
        Ok(total)
    }

    fn build_live_summary(&self) -> Result<String, String> {
        let mut parts = Vec::new();
        if let Ok(entity_summary) = crate::entity::live_summary(self.db, &self.db.data_dir) {
            parts.push(entity_summary);
        }
        if parts.is_empty() {
            parts.push("Aucune entité métier enregistrée — déclarer le registre dans Paramètres > Entités.".into());
        }
        Ok(parts.join("\n"))
    }

    fn system_prompt(&self, rag_context: &str, experience_hints: &str) -> String {
        let app_name = crate::entity::branding::ecosystem_name(&self.db.data_dir);
        let web_on = web_search::is_enabled(&self.db.data_dir);
        let exp_block = if experience_hints.is_empty() {
            String::new()
        } else {
            format!("\n\n{experience_hints}\n")
        };
        let mode_line = if web_on {
            format!(
                "Tu es Loggy, l'assistant IA de {app_name} (français). Données locales hors ligne + recherche Internet si besoin."
            )
        } else {
            format!(
                "Tu es Loggy, l'assistant IA local de {app_name} (français, hors ligne — pas d'Internet)."
            )
        };
        let web_block = if web_on {
            r#"
RECHERCHE INTERNET (activée) :
- Infos externes (actualités, définitions, météo, culture générale) : {"tool":"web_search","params":{"query":"mots-clés français"},"explain":"..."}
- Ne pas confondre avec dda_list (données locales entités uniquement).
"#
        } else {
            ""
        };
        format!(
            r#"{mode_line}

ARCHITECTURE ENTITÉS (prioritaire) :
- Les entités métier sont déclarées dans Paramètres > Entités (registry.json) : nom + attributs typés.
- Chaque entité génère automatiquement table SQLite, formulaire modal, liste tableau sur le TABLEAU DE BORD.
- NE PAS créer de nouvel écran ni de menu. L'utilisateur gère les données via le tableau de bord (ex. « gérer les users »).
- CRUD : dda_list / dda_create / dda_update / dda_delete / dda_get avec screen_key = nom exact de l'entité.
- Consulter MASTER_entities_schema.txt et MASTER_entities_tools.txt (RAG auto).

INTERDIT : LaTeX (\documentclass), code inventé, tutoriels génériques. Pas de module biens/finances immobilier.
Salutations (bonjour, merci, au revoir) : réponds en français naturel, sans JSON ni outil.
Pour INTERROGER ou MODIFIER les données entités, utilise UNIQUEMENT un JSON sur une ligne:
{{"tool":"nom_outil","params":{{...}},"explain":"raison"}}

Outils clés : dda_list, dda_get, dda_create, dda_update, dda_delete — screen_key = nom d'entité exact.{web_block}
Après exécution d'outil : courte phrase en français + données en ```json si pertinent.
- Réponse LISTE → tableau JSON ; réponse FICHE → un seul objet JSON.
Écriture (dda_create, dda_update, dda_delete) = confirmation humaine obligatoire.
{exp_block}
Contexte:
{rag_context}
"#
        )
    }

    pub fn chat(
        &self,
        user: &SessionUser,
        conversation_id: Option<&str>,
        user_message: &str,
    ) -> Result<ChatReply, String> {
        let conv_id = conversation_id
            .map(|s| s.to_string())
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        if conversation_id.is_none() {
            let title: String = user_message.chars().take(48).collect();
            self.db
                .ai_create_conversation(&conv_id, &user.id, &title)
                .map_err(|e| e.to_string())?;
        }

        let chunk_count: i64 = self
            .db
            .conn
            .query_row("SELECT COUNT(*) FROM ai_chunks", [], |r| r.get(0))
            .unwrap_or(0);
        if chunk_count == 0 {
            self.reindex()?;
        }

        self.db
            .ai_add_message(&conv_id, "user", user_message)
            .map_err(|e| e.to_string())?;

        if let Some(result) = try_translate_previous_reply(self.db, &conv_id, user_message) {
            return match result {
                Ok(msg) => self.store_assistant_reply(&conv_id, user_message, msg, vec![]),
                Err(e) => self.store_assistant_reply(&conv_id, user_message, e, vec![]),
            };
        }

        let effective = core_message(user_message);

        if let Some(kind) = classify_greeting(&effective) {
            let stats = self.db.dashboard_stats().ok();
            let app_name = crate::entity::branding::ecosystem_name(&self.db.data_dir);
            let msg = format_greeting_reply(kind, &effective, stats.as_ref(), &app_name);
            return self.store_assistant_reply(&conv_id, user_message, msg, vec![]);
        }

        if let Some(msg) = try_quick_answer(&effective) {
            return self.store_assistant_reply(&conv_id, user_message, msg, vec![]);
        }

        if let Some((msg, tr)) =
            self.try_direct_web_research_intent(&conv_id, &effective, &user.privileges)
        {
            return self.store_assistant_reply(&conv_id, user_message, msg, vec![tr]);
        }

        let experience_hints = format_experience_hints(self.db, &effective, 4);

        // Listes / créations : pas de RAG doc (évite réponses hors sujet du 7B local).
        let rag_context = if wants_action_intent(&effective) {
            self.build_live_summary()?
        } else if wants_internet_research_intent(&effective) {
            "Question générale — pas de données métier locales.".to_string()
        } else {
            let rag = RagStore::new(self.db);
            let fts_query = rewrite_user_query_for_fts(self.db, &effective);
            let mut chunks = rag.search(&fts_query, 3).unwrap_or_default();
            if chunks.is_empty() && fts_query != effective {
                chunks = rag.search(&effective, 3).unwrap_or_default();
            }
            if chunks.is_empty() {
                self.build_live_summary()?
            } else {
                chunks.join("\n---\n")
            }
        };

        if let Some((msg, tr)) =
            self.try_direct_delete_all_intent(&conv_id, &effective, &user.privileges)
        {
            return self.store_assistant_reply(&conv_id, user_message, msg, vec![tr]);
        }

        if let Some((msg, tr)) =
            self.try_direct_delete_intent(&conv_id, &effective, &user.privileges)
        {
            return self.store_assistant_reply(&conv_id, user_message, msg, vec![tr]);
        }

        if let Some((msg, tr)) =
            self.try_direct_update_intent(&conv_id, &effective, &user.privileges)
        {
            return self.store_assistant_reply(&conv_id, user_message, msg, vec![tr]);
        }

        if let Some((msg, tr)) =
            self.try_direct_write_intent(&conv_id, &effective, &user.privileges)
        {
            return self.store_assistant_reply(&conv_id, user_message, msg, vec![tr]);
        }

        if let Some((msg, tr)) = self.try_direct_list_intent(&effective, &user.privileges) {
            return self.store_assistant_reply(&conv_id, user_message, msg, vec![tr]);
        }

        if let Some((msg, tr)) = self.try_direct_export_intent(&effective, &user.privileges) {
            return self.store_assistant_reply(&conv_id, user_message, msg, vec![tr]);
        }

        if let Some((msg, tr)) =
            self.try_direct_generate_loyers_intent(&effective, &user.privileges)
        {
            return self.store_assistant_reply(&conv_id, user_message, msg, vec![tr]);
        }

        if let Some((msg, tr)) = self.try_direct_detail_intent(&effective, &user.privileges) {
            return self.store_assistant_reply(&conv_id, user_message, msg, vec![tr]);
        }

        if let Some((msg, tr)) =
            self.try_direct_pay_finance_intent(&conv_id, &effective, &user.privileges)
        {
            return self.store_assistant_reply(&conv_id, user_message, msg, vec![tr]);
        }

        if let Some((msg, tr)) = self.try_direct_search_intent(&effective, &user.privileges) {
            return self.store_assistant_reply(&conv_id, user_message, msg, vec![tr]);
        }

        if let Some((msg, tr, _)) =
            try_experience_intent(self.db, &conv_id, &effective, &user.privileges)
        {
            return self.store_assistant_reply(&conv_id, user_message, msg, vec![tr]);
        }

        let history = self
            .db
            .ai_list_messages(&conv_id, 12)
            .map_err(|e| e.to_string())?;

        let mut messages = vec![ChatMessage {
            role: "system".into(),
            content: self.system_prompt(&rag_context, &experience_hints),
        }];
        for m in &history {
            messages.push(ChatMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            });
        }

        let mut tool_results = Vec::new();
        let _ = LlamaServer::prepare(self.db, false);
        let mut reply_text = match LlamaServer::chat(Some(self.db), messages) {
            Ok(t) => t,
            Err(e) => {
                if let Some((msg, tr)) =
                    self.try_direct_write_intent(&conv_id, &effective, &user.privileges)
                {
                    return self.store_assistant_reply(&conv_id, user_message, msg, vec![tr]);
                }
                if let Some((msg, tr)) =
                    self.try_direct_list_intent(&effective, &user.privileges)
                {
                    return self.store_assistant_reply(&conv_id, user_message, msg, vec![tr]);
                }
                return Err(e);
            }
        };

        if let Some(call) = parse_tool_call(&reply_text) {
            let tr = self.handle_tool(&conv_id, &user.privileges, &call)?;
            tool_results.push(tr.clone());
            if tr.requires_confirmation {
                reply_text = tr.message.clone();
            } else if tr.success {
                record_success(self.db, &effective, &call.tool, &call.params, "success");
                reply_text = format_tool_reply(&call.tool, &tr);
            } else {
                reply_text = tr.message.clone();
            }
        } else if let Some((msg, tr)) =
            self.try_direct_write_intent(&conv_id, &effective, &user.privileges)
        {
            reply_text = msg;
            tool_results.push(tr);
        } else if let Some((msg, tr)) = self.try_direct_list_intent(&effective, &user.privileges)
        {
            reply_text = msg;
            tool_results.push(tr);
        } else if reply_text.contains("\\documentclass")
            || reply_text.contains("\\begin{document}")
        {
            if let Some((msg, tr)) = self.try_direct_export_intent(&effective, &user.privileges) {
                reply_text = msg;
                tool_results.push(tr);
            } else {
                let app = crate::entity::branding::ecosystem_name(&self.db.data_dir);
                reply_text = format!(
                    "Je ne génère pas de LaTeX. Demandez une action métier — j'utiliserai les outils {app}."
                );
            }
        }

        self.store_assistant_reply(&conv_id, user_message, reply_text, tool_results)
    }

    /// Suppression massive — confirmation réservée au Directeur.
    fn try_direct_delete_all_intent(
        &self,
        conversation_id: &str,
        user_message: &str,
        privileges: &[String],
    ) -> Option<(String, ToolResult)> {
        let call = parse_delete_all_from_user_message(user_message).or_else(|| {
            parse_tool_call(user_message).filter(|c| c.tool == "delete_all_biens")
        })?;
        let tr = self.handle_tool(conversation_id, privileges, &call).ok()?;
        let msg = if tr.requires_confirmation {
            format!(
                "{}\n\nCette action est irréversible. Seul un **Directeur** peut confirmer.",
                tr.message
            )
        } else {
            tr.message.clone()
        };
        record_success(
            self.db,
            user_message,
            &call.tool,
            &call.params,
            if tr.requires_confirmation {
                "pending"
            } else {
                "success"
            },
        );
        Some((msg, tr))
    }

    /// Suppression par référence (confirmation obligatoire).
    fn try_direct_delete_intent(
        &self,
        conversation_id: &str,
        user_message: &str,
        privileges: &[String],
    ) -> Option<(String, ToolResult)> {
        let call = parse_delete_from_user_message(user_message)
            .or_else(|| {
                parse_tool_call(user_message).filter(|c| {
                    c.tool == "delete_bien" || c.tool == "delete_hangar"
                })
            })?;
        let tr = self.handle_tool(conversation_id, privileges, &call).ok()?;
        let msg = if tr.requires_confirmation {
            format!(
                "{}\n\nCette action est irréversible après confirmation.",
                tr.message
            )
        } else {
            tr.message.clone()
        };
        record_success(
            self.db,
            user_message,
            &call.tool,
            &call.params,
            if tr.requires_confirmation {
                "pending"
            } else {
                "success"
            },
        );
        Some((msg, tr))
    }

    /// Création / écriture : JSON dans le message ou outil explicite (sans llama).
    fn try_direct_update_intent(
        &self,
        conversation_id: &str,
        user_message: &str,
        privileges: &[String],
    ) -> Option<(String, ToolResult)> {
        let call = parse_update_from_user_message(user_message)
            .or_else(|| {
                parse_crud_from_user_message(user_message).filter(|c| c.tool.starts_with("update_"))
            })
            .or_else(|| {
                parse_tool_call(user_message).filter(|c| c.tool.starts_with("update_"))
            })?;
        let tr = self.handle_tool(conversation_id, privileges, &call).ok()?;
        let msg = if tr.requires_confirmation {
            format!(
                "{}\n\nConfirmez pour appliquer la modification sur `{}`.",
                tr.message,
                call.params
                    .get("reference")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?")
            )
        } else if tr.success {
            format_tool_reply(&call.tool, &tr)
        } else {
            tr.message.clone()
        };
        record_success(
            self.db,
            user_message,
            &call.tool,
            &call.params,
            if tr.requires_confirmation {
                "pending"
            } else {
                "success"
            },
        );
        Some((msg, tr))
    }

    fn try_direct_write_intent(
        &self,
        conversation_id: &str,
        user_message: &str,
        privileges: &[String],
    ) -> Option<(String, ToolResult)> {
        let call = parse_crud_from_user_message(user_message)
            .or_else(|| parse_create_from_user_message(user_message))
            .or_else(|| parse_tool_call(user_message).filter(|c| is_write_tool(&c.tool)))?;
        if !is_write_tool(&call.tool) {
            return None;
        }
        let tr = self.handle_tool(conversation_id, privileges, &call).ok()?;
        let msg = if tr.requires_confirmation {
            format!(
                "{}\n\nConfirmez pour enregistrer `{}` en base.",
                tr.message,
                call.params
                    .get("reference")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?")
            )
        } else if tr.success {
            format_tool_reply(&call.tool, &tr)
        } else {
            tr.message.clone()
        };
        record_success(
            self.db,
            user_message,
            &call.tool,
            &call.params,
            if tr.requires_confirmation {
                "pending"
            } else {
                "success"
            },
        );
        Some((msg, tr))
    }

    fn try_direct_export_intent(
        &self,
        user_message: &str,
        privileges: &[String],
    ) -> Option<(String, ToolResult)> {
        if !wants_export_intent(user_message) {
            return None;
        }
        let tool = infer_export_tool(user_message)?;
        let mut params = serde_json::json!({});
        if tool == "export_bien_report" && wants_last_bien(user_message) {
            params["use_last"] = serde_json::Value::Bool(true);
        }
        if tool == "export_finances_month" {
            if let Some((annee, mois)) = infer_period_from_message(user_message) {
                params["annee"] = serde_json::json!(annee);
                params["mois"] = serde_json::json!(mois);
            }
        }
        if let Some(reference) = crate::ai::tools::extract_reference_from_message(user_message) {
            params["reference"] = serde_json::Value::String(reference);
        }
        let call = ToolCall {
            tool: tool.to_string(),
            params,
            explain: None,
        };
        let tr = execute_read_tool(self.db, privileges, &call).ok()?;
        if !tr.success {
            return None;
        }
        record_success(self.db, user_message, tool, &call.params, "success");
        Some((format_tool_reply(tool, &tr), tr))
    }

    fn try_direct_generate_loyers_intent(
        &self,
        user_message: &str,
        privileges: &[String],
    ) -> Option<(String, ToolResult)> {
        if !wants_generate_loyers_intent(user_message) {
            return None;
        }
        let (annee, mois) = infer_period_from_message(user_message)?;
        let call = ToolCall {
            tool: "finances_generate_month".into(),
            params: serde_json::json!({ "annee": annee, "mois": mois }),
            explain: None,
        };
        let tr = execute_read_tool(self.db, privileges, &call).ok()?;
        if !tr.success {
            return None;
        }
        record_success(self.db, user_message, "finances_generate_month", &call.params, "success");
        Some((format_tool_reply("finances_generate_month", &tr), tr))
    }

    fn try_direct_detail_intent(
        &self,
        user_message: &str,
        privileges: &[String],
    ) -> Option<(String, ToolResult)> {
        if !wants_detail_intent(user_message) && !wants_last_bien(user_message) {
            return None;
        }
        if wants_export_intent(user_message) {
            return None;
        }
        let mut params = serde_json::json!({});
        if wants_last_bien(user_message) {
            params["use_last"] = serde_json::Value::Bool(true);
        } else if let Some(reference) = crate::ai::tools::extract_reference_from_message(user_message)
        {
            params["reference"] = serde_json::Value::String(reference);
        }
        let call = ToolCall {
            tool: "get_bien".into(),
            params,
            explain: None,
        };
        let tr = execute_read_tool(self.db, privileges, &call).ok()?;
        if !tr.success {
            return None;
        }
        record_success(self.db, user_message, "get_bien", &call.params, "success");
        Some((format_tool_reply("get_bien", &tr), tr))
    }

    fn try_direct_pay_finance_intent(
        &self,
        conversation_id: &str,
        user_message: &str,
        privileges: &[String],
    ) -> Option<(String, ToolResult)> {
        if !wants_pay_finance_intent(user_message) {
            return None;
        }
        let reference = crate::ai::tools::extract_reference_from_message(user_message)?;
        let call = ToolCall {
            tool: "validate_finance".into(),
            params: serde_json::json!({ "reference": reference }),
            explain: Some("Marquer l'écriture comme payée".into()),
        };
        let tr = self.handle_tool(conversation_id, privileges, &call).ok()?;
        let msg = if tr.requires_confirmation {
            format!("{}\n\nConfirmez pour enregistrer le paiement.", tr.message)
        } else {
            tr.message.clone()
        };
        record_success(
            self.db,
            user_message,
            "validate_finance",
            &call.params,
            if tr.requires_confirmation {
                "pending"
            } else {
                "success"
            },
        );
        Some((msg, tr))
    }

    fn try_direct_web_research_intent(
        &self,
        conversation_id: &str,
        user_message: &str,
        privileges: &[String],
    ) -> Option<(String, ToolResult)> {
        if !web_search::is_enabled(&self.db.data_dir) {
            return None;
        }
        if !has_privilege(privileges, "ai:utiliser") {
            return None;
        }
        if !wants_internet_research_intent(user_message) {
            return None;
        }
        let query = extract_web_search_query(user_message)?;
        let result = web_search::search(&self.db.data_dir, &query).ok()?;
        let answer = web_search::synthesize_answer(self.db, user_message, &result).ok()?;
        let tr = ToolResult {
            tool: "web_search".into(),
            success: true,
            message: answer.clone(),
            data: Some(serde_json::to_value(&result).unwrap_or(serde_json::Value::Null)),
            requires_confirmation: false,
            pending_id: None,
            confirm_privilege: None,
        };
        record_success(
            self.db,
            user_message,
            "web_search",
            &serde_json::json!({ "query": query }),
            "success",
        );
        let _ = conversation_id;
        Some((answer, tr))
    }

    fn try_direct_search_intent(
        &self,
        user_message: &str,
        privileges: &[String],
    ) -> Option<(String, ToolResult)> {
        if !wants_search_intent(user_message) {
            return None;
        }
        let query = extract_search_query(user_message)?;
        let call = ToolCall {
            tool: "search_biens".into(),
            params: serde_json::json!({ "query": query }),
            explain: None,
        };
        let tr = execute_read_tool(self.db, privileges, &call).ok()?;
        if !tr.success {
            return None;
        }
        record_success(self.db, user_message, "search_biens", &call.params, "success");
        Some((format_tool_reply("search_biens", &tr), tr))
    }

    /// Liste directe en base si la demande est explicite (sans 2e appel llama).
    fn try_direct_list_intent(
        &self,
        user_message: &str,
        privileges: &[String],
    ) -> Option<(String, ToolResult)> {
        if !wants_list_intent(user_message) {
            return None;
        }
        let tool = infer_list_tool(user_message)?;
        let mut params = serde_json::json!({});
        if let Some(statut) = infer_statut_param(tool, user_message) {
            params["statut"] = serde_json::Value::String(statut);
        }
        let call = ToolCall {
            tool: tool.to_string(),
            params,
            explain: None,
        };
        let tr = execute_read_tool(self.db, privileges, &call).ok()?;
        if !tr.success {
            return None;
        }
        record_success(self.db, user_message, tool, &call.params, "success");
        Some((format_tool_reply(tool, &tr), tr))
    }

    fn handle_tool(
        &self,
        conversation_id: &str,
        privileges: &[String],
        call: &ToolCall,
    ) -> Result<ToolResult, String> {
        if is_write_tool(&call.tool) {
            queue_write_action(self.db, conversation_id, call, privileges)
        } else {
            execute_read_tool(self.db, privileges, call)
        }
    }
}

fn extract_search_query(message: &str) -> Option<String> {
    let n = crate::ai::intent_filters::normalize_message(message);
    if let Some(reference) = crate::ai::tools::extract_reference_from_message(message) {
        return Some(reference);
    }
    for prefix in [
        "cherche ",
        "chercher ",
        "recherche ",
        "rechercher ",
        "trouve ",
        "trouver ",
    ] {
        if let Some(rest) = n.strip_prefix(prefix) {
            let q = rest.trim();
            if q.len() >= 2 {
                return Some(q.to_string());
            }
        }
    }
    None
}
