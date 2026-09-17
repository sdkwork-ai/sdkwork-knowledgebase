import { useMemo } from "react";

import { ScreenState } from "@sdkwork/knowledgebase-h5-commons";

import { KNOWLEDGEBASE_H5_ROUTE_REGISTRY } from "./navigation/routeRegistry";

export interface KnowledgebaseH5RouteContribution {
  id: string;
  render: () => React.ReactNode;
  titleKey: string;
}

export interface KnowledgebaseAppShellProps {
  contributions: KnowledgebaseH5RouteContribution[];
}

/**
 * H5 application shell.
 *
 * Owns navigation container composition, route contribution assembly, and app
 * layout. It must not own business SDK orchestration
 * (`APP_CLIENT_ARCHITECTURE_ALIGNMENT_SPEC.md` section 4).
 */
export function KnowledgebaseAppShell({ contributions }: KnowledgebaseAppShellProps) {
  const active = useMemo(() => contributions[0], [contributions]);

  return (
    <div className="flex min-h-full flex-col" data-testid="knowledgebase-h5-shell">
      <header className="flex items-center justify-between border-b px-4 py-3">
        <h1 className="text-base font-semibold">SDKWork Knowledgebase</h1>
      </header>
      <nav aria-label="Knowledgebase sections" className="flex gap-4 border-b px-4 py-2 text-sm">
        {KNOWLEDGEBASE_H5_ROUTE_REGISTRY.filter((entry) => entry.id !== 'app.intelligence.knowledgebase.launch').map(
          (entry) => (
            <a key={entry.id} href={`#${entry.path}`} data-route-id={entry.id}>
              {entry.id.split('.').pop()}
            </a>
          ),
        )}
      </nav>
      <main className="flex min-h-0 flex-1 flex-col">
        {active ? active.render() : (
          <ScreenState title="No capability routes registered" />
        )}
      </main>
    </div>
  );
}
