import { Suspense, lazy } from "react";
import { HashRouter, Navigate, Route, Routes } from "react-router-dom";

import { AuthGate } from "./components/AuthGate";
import { createH5RouteContributions } from "./bootstrap/routes";

const KnowledgebaseAppShell = lazy(() =>
  import("@sdkwork/knowledgebase-h5-shell").then((module) => ({ default: module.KnowledgebaseAppShell })),
);

export default function App() {
  const contributions = createH5RouteContributions();

  return (
    <AuthGate>
      <HashRouter>
        <Suspense fallback={<div role="status">Loading SDKWork Knowledgebase</div>}>
          <Routes>
            <Route path="/" element={<KnowledgebaseAppShell contributions={contributions} />} />
            <Route path="*" element={<Navigate to="/" replace />} />
          </Routes>
        </Suspense>
      </HashRouter>
    </AuthGate>
  );
}
