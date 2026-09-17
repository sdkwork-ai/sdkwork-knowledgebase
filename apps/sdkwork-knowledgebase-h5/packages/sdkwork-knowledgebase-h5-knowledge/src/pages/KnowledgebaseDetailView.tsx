import { useState } from "react";

import { LoadingState, ScreenState } from "@sdkwork/knowledgebase-h5-commons";

import type { KnowledgeRouteParams } from "../models/knowledgeModels";

export interface KnowledgebaseDetailViewProps {
  params?: KnowledgeRouteParams;
  title: string;
}

/**
 * Knowledgebase detail screen.
 *
 * Pages render services/state results; they never construct SDK clients or call
 * raw HTTP (`APP_CLIENT_ARCHITECTURE_ALIGNMENT_SPEC.md` sections 4 and 5).
 */
export function KnowledgebaseDetailView({ params, title }: KnowledgebaseDetailViewProps) {
  const [loading] = useState(false);

  if (loading) {
    return <LoadingState label={title} />;
  }

  return (
    <section data-testid="knowledgebase-h5-detail" data-space-id={params?.spaceId ?? ""}>
      <h2 className="px-4 py-3 text-sm font-semibold">{title}</h2>
      <ScreenState title={title} />
    </section>
  );
}
