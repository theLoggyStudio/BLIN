import { BrowserRouter, Navigate, Route, Routes } from "react-router-dom";
import { AuthProvider, useAuth } from "@/contexts/AuthContext";
import { EntityBrandingProvider } from "@/contexts/EntityBrandingContext";
import { UiThemeProvider } from "@/contexts/UiThemeContext";
import { AppLayout } from "@/components/layout/AppLayout";
import { OpenWindowsProvider } from "@/contexts/OpenWindowsContext";
import { DashboardChatProvider } from "@/contexts/DashboardChatContext";
import { ForcePasswordChangeModal } from "@/items/ForcePasswordChangeModal";
import { ForbiddenPage } from "@/pages/Forbidden/ForbiddenPage";
import { LoginPage } from "@/pages/Login/LoginPage";
import { DashboardPage } from "@/pages/Dashboard/DashboardPage";
import { ParametresPage } from "@/pages/system/ParametresPage";
import { RequireAnyPrivilege } from "@/components/RequirePrivilege";
import { PARAMETRES_VISIBILITY_PRIVILEGES } from "@/lib/parametresPrivileges";
import { TachesModalProvider } from "@/contexts/TachesModalContext";
import { StockModalProvider } from "@/contexts/StockModalContext";
import { EntityDefLoggyModalProvider } from "@/contexts/EntityDefLoggyModalContext";
import { AlertProvider } from "@/contexts/AlertContext";
import { AppStartupHost } from "@/items/AppStartupHost";
import { AiRuntimeSetupGate } from "@/items/AiRuntimeSetupGate";

function LoadingScreen() {
  return (
    <div className="flex min-h-screen items-center justify-center bg-background">
      <div className="h-10 w-10 animate-spin rounded-full border-2 border-secondary border-t-transparent" />
    </div>
  );
}

function ProtectedRoutes() {
  const { user, loading, mustChangePassword } = useAuth();

  if (loading) {
    return <LoadingScreen />;
  }

  if (!user) {
    return <LoginPage />;
  }

  if (mustChangePassword) {
    return (
      <div className="flex min-h-screen items-center justify-center bg-background">
        <ForcePasswordChangeModal />
      </div>
    );
  }

  return (
    <Routes>
      <Route
        element={
          <OpenWindowsProvider>
            <DashboardChatProvider>
              <TachesModalProvider>
                <StockModalProvider>
                  <EntityDefLoggyModalProvider>
                    <AppLayout />
                  </EntityDefLoggyModalProvider>
                </StockModalProvider>
              </TachesModalProvider>
            </DashboardChatProvider>
          </OpenWindowsProvider>
        }
      >
        <Route index element={<DashboardPage />} />
        <Route
          path="parametres"
          element={
            <RequireAnyPrivilege privileges={PARAMETRES_VISIBILITY_PRIVILEGES}>
              <ParametresPage />
            </RequireAnyPrivilege>
          }
        />
      </Route>
      <Route path="interdit" element={<ForbiddenPage />} />
      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  );
}

function AppRoutes() {
  return (
    <Routes>
      <Route path="/*" element={<ProtectedRoutes />} />
    </Routes>
  );
}

export default function App() {
  return (
    <BrowserRouter>
      <UiThemeProvider>
        <AuthProvider>
          <EntityBrandingProvider>
            <AlertProvider>
              <AiRuntimeSetupGate>
                <AppStartupHost />
                <AppRoutes />
              </AiRuntimeSetupGate>
            </AlertProvider>
          </EntityBrandingProvider>
        </AuthProvider>
      </UiThemeProvider>
    </BrowserRouter>
  );
}
