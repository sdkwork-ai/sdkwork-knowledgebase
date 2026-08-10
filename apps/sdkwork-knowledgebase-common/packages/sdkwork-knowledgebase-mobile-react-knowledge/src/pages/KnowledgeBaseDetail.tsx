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

export function KnowledgeBaseDetail() {
  const { t } = useTranslation("knowledge");
  const navigate = useNavigate();
  const { kbId, id: documentId } = useParams<{ kbId: string; id: string }>();

  const [document, setDocument] = useState<KnowledgeDocument | null>(null);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState(false);
  const [notFound, setNotFound] = useState(false);

  const [menuOpen, setMenuOpen] = useState(false);
  const [renameOpen, setRenameOpen] = useState(false);
  const [deleteOpen, setDeleteOpen] = useState(false);

  const load = useCallback(async () => {
    if (!documentId) {
      return;
    }
    setLoading(true);
    setLoadError(false);
    try {
      const doc = await KnowledgeBaseService.getDocument(documentId);
      if (!doc) {
        setNotFound(true);
        return;
      }
      setDocument(doc);
    } catch {
      setLoadError(true);
    } finally {
      setLoading(false);
    }
  }, [documentId]);

  useEffect(() => {
    void load();
  }, [load]);

  const handleRenameConfirm = async (value: string) => {
    if (!document) {
      return;
    }
    try {
      const updated = await KnowledgeBaseService.updateDocument(document.id, {
        title: value,
      });
      if (updated) {
        setDocument((current) => (current ? { ...current, title: value } : current));
        showToast(t("doc_rename_success", "已重命名"));
      } else {
        showToast(t("doc_not_found", "文档不存在"));
        setNotFound(true);
      }
    } catch {
      showToast(t("doc_rename_failed", "重命名失败，请重试"));
    } finally {
      setRenameOpen(false);
    }
  };

  const handleDeleteConfirm = async () => {
    if (!document) {
      return;
    }
    try {
      await KnowledgeBaseService.deleteDocument(document.id);
      showToast(t("doc_delete_success", "已删除"));
      navigate(`/workspace/knowledge/${kbId}`, { replace: true });
    } catch {
      showToast(t("doc_delete_failed", "删除失败，请重试"));
    } finally {
      setDeleteOpen(false);
    }
  };

  if (notFound) {
    return (
      <PageLayout title={t("doc_detail", "文档详情")}>
        <div className="flex flex-col items-center justify-center flex-1 px-6 text-center">
          <p className="text-[15px] text-text-sub mb-4">
            {t("doc_not_found", "文档不存在或已被删除")}
          </p>
          <button
            onClick={() => navigate(`/workspace/knowledge/${kbId}`, { replace: true })}
            className="px-5 py-2.5 rounded-xl bg-primary-blue text-white text-[14px] font-medium active:scale-95 transition-transform"
          >
            {t("back_to_list", "返回列表")}
          </button>
        </div>
      </PageLayout>
    );
  }

  return (
    <PageLayout
      title={t("doc_detail", "文档详情")}
      rightElement={
        <button
          onClick={() => setMenuOpen(true)}
          className="p-2 rounded-full hover:bg-black/5 dark:hover:bg-white/5 active:scale-95 transition-all"
          aria-label={t("menu_more", "更多操作")}
        >
          <MoreHorizontal className="w-6 h-6 text-text-main" strokeWidth={2.5} />
        </button>
      }
    >
      {loading ? (
        <div className="flex flex-col items-center justify-center py-20 text-text-sub">
          <RefreshCw className="w-6 h-6 animate-spin mb-3" />
          <span className="text-[14px]">{t("loading", "加载中...")}</span>
        </div>
      ) : loadError ? (
        <div className="flex flex-col items-center justify-center py-20 text-center px-6">
          <p className="text-[14px] text-text-sub mb-4">
            {t("load_error", "加载失败，请稍后重试")}
          </p>
          <button
            onClick={() => void load()}
            className="px-5 py-2.5 rounded-xl bg-primary-blue text-white text-[14px] font-medium active:scale-95 transition-transform"
          >
            {t("retry", "重试")}
          </button>
        </div>
      ) : document ? (
        <div className="flex-1 overflow-y-auto p-5">
          <h1 className="text-[22px] font-bold text-text-main leading-snug mb-3">
            {document.title}
          </h1>

          {document.contentState && document.contentState !== "ready" && (
            <div className="flex items-center gap-2 bg-amber-500/10 border border-amber-500/20 text-amber-600 dark:text-amber-400 rounded-xl px-3.5 py-2.5 mb-4">
              <Clock className="w-4 h-4 shrink-0" />
              <span className="text-[13px]">
                {t("content_syncing", "内容同步中，请稍后刷新查看全文")}
              </span>
            </div>
          )}

          {document.content && document.content.trim().length > 0 ? (
            <div className="pb-6">{renderMarkdownContent(document.content)}</div>
          ) : document.contentState && document.contentState !== "ready" ? null : (
            <p className="text-[14px] text-text-sub pb-6">
              {t("doc_empty_content", "暂无内容")}
            </p>
          )}
        </div>
      ) : null}

      <ActionSheet
        isOpen={menuOpen}
        onClose={() => setMenuOpen(false)}
        title={document?.title}
        options={[
          {
            label: t("menu_rename", "重命名"),
            onClick: () => setRenameOpen(true),
          },
          {
            label: t("menu_delete", "删除"),
            danger: true,
            onClick: () => setDeleteOpen(true),
          },
        ]}
      />

      <RenameDialog
        isOpen={renameOpen}
        title={t("doc_rename_title", "重命名文档")}
        initialValue={document?.title ?? ""}
        placeholder={t("doc_title_placeholder", "文档标题")}
        confirmText={t("save", "保存")}
        onConfirm={(value) => void handleRenameConfirm(value)}
        onCancel={() => setRenameOpen(false)}
      />

      <ConfirmDialog
        isOpen={deleteOpen}
        title={t("doc_delete_title", "删除文档")}
        message={t("doc_delete_confirm", "确定删除该文档吗？此操作不可恢复。")}
        confirmText={t("confirm_delete", "删除")}
        onConfirm={() => void handleDeleteConfirm()}
        onCancel={() => setDeleteOpen(false)}
      />
    </PageLayout>
  );
}
