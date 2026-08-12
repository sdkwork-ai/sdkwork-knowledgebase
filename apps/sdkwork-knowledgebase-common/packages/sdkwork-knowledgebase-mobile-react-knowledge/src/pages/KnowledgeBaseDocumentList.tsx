import React, { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useParams } from "react-router";
import { FilePlus2, MoreHorizontal, RefreshCw } from "lucide-react";
import { ActionSheet, PageLayout, showToast } from "@sdkwork/ui-mobile-react";

import {
  KnowledgeBase,
  KnowledgeBaseService,
  KnowledgeDocument,
} from "../services/KnowledgeBaseService";
import { KnowledgeDocumentCard } from "../components/KnowledgeDocumentCard";
import { ConfirmDialog } from "../components/ConfirmDialog";
import { RenameDialog } from "../components/RenameDialog";

import { CapabilityUnavailablePage } from "../components/CapabilityUnavailablePage";

export function KnowledgeBaseDocumentList() {
  return (
    <CapabilityUnavailablePage />
  );
}
