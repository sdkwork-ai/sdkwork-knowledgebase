import { useEffect, useState, type ReactNode } from "react";

import {
  isKnowledgebaseH5SessionAuthenticated,
  KNOWLEDGEBASE_H5_SESSION_CHANGED_EVENT,
} from "@sdkwork/knowledgebase-h5-core/session";

interface AuthGateProps {
  children: ReactNode;
}

/**
 * Shell-owned authentication gate.
 *
 * Route guards are shell/runtime responsibilities
 * (`APP_CLIENT_ARCHITECTURE_ALIGNMENT_SPEC.md` section 7); the capability package
 * only declares auth hints. The IAM session itself is owned by the appbase
 * runtime wired in `src/bootstrap/iamRuntime.ts`.
 */
export function AuthGate({ children }: AuthGateProps) {
  const [authenticated, setAuthenticated] = useState(() => isKnowledgebaseH5SessionAuthenticated());

  useEffect(() => {
    const handler = () => setAuthenticated(isKnowledgebaseH5SessionAuthenticated());
    window.addEventListener(KNOWLEDGEBASE_H5_SESSION_CHANGED_EVENT, handler);
    return () => window.removeEventListener(KNOWLEDGEBASE_H5_SESSION_CHANGED_EVENT, handler);
  }, []);

  if (!authenticated) {
    return (
      <div data-testid="knowledgebase-h5-auth-gate" style={{ padding: 16 }}>
        <h1>SDKWork Knowledgebase</h1>
        <p>Sign in through the application IAM bootstrap to continue.</p>
      </div>
    );
  }

  return <>{children}</>;
}
