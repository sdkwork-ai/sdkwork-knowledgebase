import React, { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useParams } from "react-router";
import { Clock, MoreHorizontal, RefreshCw } from "lucide-react";
import { ActionSheet, PageLayout, showToast } from "@sdkwork/ui-mobile-react";

import {
  KnowledgeBaseService,
  KnowledgeDocument,
} from "../services/KnowledgeBaseService";
import { ConfirmDialog } from "../components/ConfirmDialog";
import { RenameDialog } from "../components/RenameDialog";

function renderMarkdownContent(content: string): React.ReactNode[] {
  const lines = content.split("\n");
  const blocks: React.ReactNode[] = [];
  let inCodeBlock = false;
  let codeLines: string[] = [];

  const flushCode = (key: string) => {
    if (codeLines.length > 0) {
      blocks.push(
        <pre
          key={key}
          className="bg-[#f3f4f6] dark:bg-[#2c2d2e] rounded-xl p-4 text-[13px] leading-relaxed overflow-x-auto my-2"
        >
          {codeLines.join("\n")}
        </pre>,
      );
      codeLines = [];
    }
  };

  lines.forEach((line, index) => {
    const key = `line-${index}`;
    if (line.trim().startsWith("```")) {
      if (inCodeBlock) {
        inCodeBlock = false;
        flushCode(key);
      } else {
        inCodeBlock = true;
        codeLines = [];
      }
      return;
    }
    if (inCodeBlock) {
      codeLines.push(line);
      return;
    }
    const trimmed = line.trim();
    if (trimmed.length === 0) {
      return;
    }
    if (trimmed.startsWith("### ")) {
      blocks.push(
        <h4 key={key} className="text-[16px] font-semibold text-text-main mt-4 mb-1">
          {trimmed.slice(4)}
        </h4>,
      );
    } else if (trimmed.startsWith("## ")) {
      blocks.push(
        <h3 key={key} className="text-[18px] font-bold text-text-main mt-5 mb-1">
          {trimmed.slice(3)}
        </h3>,
      );
    } else if (trimmed.startsWith("# ")) {
      blocks.push(
        <h2 key={key} className="text-[21px] font-bold text-text-main mt-6 mb-2">
          {trimmed.slice(2)}
        </h2>,
      );
    } else if (trimmed.startsWith("- ") || trimmed.startsWith("* ")) {
      blocks.push(
        <p key={key} className="text-[15px] text-text-main leading-relaxed mb-1.5 pl-3">
          • {trimmed.slice(2)}
        </p>,
      );
    } else {
      blocks.push(
        <p key={key} className="text-[15px] text-text-main leading-relaxed mb-2">
          {trimmed}
        </p>,
      );
    }
  });
  if (inCodeBlock) {
    flushCode(`code-${lines.length}`);
  }
  return blocks;
}

import { CapabilityUnavailablePage } from "../components/CapabilityUnavailablePage";

export function KnowledgeBaseDetail() {
  return (
    <CapabilityUnavailablePage />
  );
}
