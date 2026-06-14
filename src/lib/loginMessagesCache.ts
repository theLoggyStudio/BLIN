export interface LoginMessagesCache {
  greeting: string;
  invalid_credentials: string;
  prepared: boolean;
}

let cache: LoginMessagesCache | null = null;

export function setLoginMessagesCache(value: LoginMessagesCache): void {
  cache = value;
}

export function getLoginMessagesCache(): LoginMessagesCache | null {
  return cache;
}

export function getInvalidCredentialsMessage(fallback = "Identifiants invalides."): string {
  return cache?.invalid_credentials?.trim() || fallback;
}

export function isLoginMessagesPrepared(): boolean {
  return Boolean(cache?.prepared);
}
