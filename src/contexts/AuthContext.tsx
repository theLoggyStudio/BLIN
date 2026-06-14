import { invoke } from "@tauri-apps/api/core";
import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import type { Privilege, User } from "@/types/auth";

interface LoginResponse {
  token: string;
  user: User;
  must_change_password: boolean;
  login_greeting?: string;
  login_notices?: string[];
}

interface AuthContextValue {
  user: User | null;
  loading: boolean;
  mustChangePassword: boolean;
  loginGreeting: string | null;
  loginNotices: string[];
  clearLoginNotices: () => void;
  login: (email: string, password: string) => Promise<void>;
  logout: () => Promise<void>;
  changePassword: (newPassword: string, confirmPassword: string) => Promise<void>;
  syncSessionPrivileges: () => Promise<void>;
  hasPrivilege: (privilege: Privilege | string) => boolean;
}

const AuthContext = createContext<AuthContextValue | null>(null);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<User | null>(null);
  const [mustChangePassword, setMustChangePassword] = useState(false);
  const [loginNotices, setLoginNotices] = useState<string[]>([]);
  const [loginGreeting, setLoginGreeting] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  const applyUser = useCallback((next: User | null) => {
    setUser(next);
    setMustChangePassword(Boolean(next?.must_change_password));
  }, []);

  const loadSession = useCallback(async () => {
    try {
      const current = await invoke<User>("auth_current_user");
      applyUser(current);
    } catch {
      applyUser(null);
    } finally {
      setLoading(false);
    }
  }, [applyUser]);

  useEffect(() => {
    void loadSession();
  }, [loadSession]);

  const login = useCallback(
    async (email: string, password: string) => {
      const response = await invoke<LoginResponse>("auth_login", {
        payload: { email, password },
      });
      applyUser(response.user);
      setLoginGreeting(response.login_greeting?.trim() || null);
      setLoginNotices(response.login_notices ?? []);
      setMustChangePassword(
        response.must_change_password || Boolean(response.user.must_change_password),
      );
    },
    [applyUser],
  );

  const logout = useCallback(async () => {
    await invoke("auth_logout");
    applyUser(null);
    setLoginNotices([]);
    setLoginGreeting(null);
  }, [applyUser]);

  const clearLoginNotices = useCallback(() => {
    setLoginNotices([]);
    setLoginGreeting(null);
  }, []);

  const changePassword = useCallback(
    async (newPassword: string, confirmPassword: string) => {
      const updated = await invoke<User>("auth_change_password", {
        payload: { new_password: newPassword, confirm_password: confirmPassword },
      });
      applyUser(updated);
    },
    [applyUser],
  );

  const syncSessionPrivileges = useCallback(async () => {
    try {
      const fresh = await invoke<User>("auth_sync_session_privileges");
      applyUser(fresh);
    } catch {
      /* session inactive */
    }
  }, [applyUser]);

  const hasPrivilege = useCallback(
    (privilege: Privilege | string): boolean => {
      if (!user) return false;
      if (user.privileges.includes("*")) return true;
      if (user.privileges.includes(privilege as Privilege)) return true;
      const [module] = privilege.split(":");
      return user.privileges.includes(`${module}:*` as Privilege);
    },
    [user],
  );

  const value = useMemo(
    () => ({
      user,
      loading,
      mustChangePassword,
      loginGreeting,
      loginNotices,
      clearLoginNotices,
      login,
      logout,
      changePassword,
      syncSessionPrivileges,
      hasPrivilege,
    }),
    [
      user,
      loading,
      mustChangePassword,
      loginGreeting,
      loginNotices,
      clearLoginNotices,
      login,
      logout,
      changePassword,
      syncSessionPrivileges,
      hasPrivilege,
    ],
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth(): AuthContextValue {
  const ctx = useContext(AuthContext);
  if (!ctx) {
    throw new Error("useAuth doit être utilisé dans AuthProvider");
  }
  return ctx;
}
