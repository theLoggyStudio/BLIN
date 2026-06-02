import { Link } from "react-router-dom";
import { ShieldX } from "lucide-react";
import { Button } from "@/components/ui/Button";
import { Panel } from "@/components/ui/Panel";

export function ForbiddenPage() {
  return (
    <div className="min-h-screen flex items-center justify-center p-6">
      <Panel title="Accès refusé" variant="accent" className="max-w-md w-full text-center">
        <ShieldX className="h-16 w-16 text-primary mx-auto mb-4" />
        <p className="text-muted text-sm mb-6">
          Votre rôle ne possède pas le privilège requis pour accéder à cette page.
          Contactez un administrateur si vous pensez qu&apos;il s&apos;agit d&apos;une erreur.
        </p>
        <Link to="/">
          <Button className="w-full">Retour à l&apos;accueil</Button>
        </Link>
      </Panel>
    </div>
  );
}
