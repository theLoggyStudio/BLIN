# Blin

Application **Blin** — architecture Tauri v2, React 19, DDA JSON, Loggy.

## Démarrage

```bash
npm install
npm run tauri dev
```

## Compte administrateur (seed par défaut)

| Champ        | Valeur              |
|--------------|---------------------|
| E-mail       | `admin@blin.local`  |
| Mot de passe | `Admin123!`         |

> Adaptez les identifiants et la charte dans **Paramètres → Entités** selon vos besoins métier.

## Logo et icônes

- Logo web / sidebar : `public/logo.png`
- Icônes fenêtre Tauri : `npm run tauri:icons` (génère depuis `public/logo.png`)
