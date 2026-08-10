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

export function KnowledgeBaseDocumentList() {
  const { t } = useTranslation("knowledge");
  const navigate = useNavigate();
  const { id: knowledgeBaseId } = useParams<{ id: string }>();

  const [kb, setKb] = useState<KnowledgeBase | null>(null);
  const [documents, setDocuments] = useState<KnowledgeDocument[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState(false);
  const [kbMissing, setKbMissing] = useState(false);

  const [searchQuery, setSearchQuery] = useState("");
  const [kbMenuOpen, setKbMenuOpen] = useState(false);
  const [renameOpen, setRenameOpen] = useState(false);
  const [deleteKbOpen, setDeleteKbOpen] = useState(false);
  const [menuDoc, setMenuDoc] = useState<KnowledgeDocument | null>(null);
  const [deleteDoc, setDeleteDoc] = useState<KnowledgeDocument | null>(null);

  const load = useCallback(async () => {
    if (!knowledgeBaseId) {
      return;
    }
    setLoading(true);
    setLoadError(false);
    try {
      const [space, docs] = await Promise.all([
        KnowledgeBaseService.getKnowledgeBase(knowledgeBaseId),
        KnowledgeBaseService.getDocumentsByKbId(knowledgeBaseId),
      ]);
      if (!space) {
        setKbMissing(true);
        return;
      }
      setKb(space);
      setDocuments(docs);
    } catch {
      setLoadError(true);
    } finally {
      setLoading(false);
    }
  }, [knowledgeBaseId]);

  useEffect(() => {
    void load();
  }, [load]);

  const filteredDocuments = useMemo(() => {
    const query = searchQuery.trim().toLowerCase();
    if (query.length === 0) {
      return documents;
    }
    return documents.filter((doc) => doc.title.toLowerCase().includes(query));
  }, [documents, searchQuery]);

  const handleRenameConfirm = async (value: string) => {
    if (!knowledgeBaseId) {
      return;
    }
    try {
      const updated = await KnowledgeBaseService.updateKnowledgeBase(knowledgeBaseId, {
        name: value,
      });
      if (updated) {
        setKb(updated);
        showToast(t("rename_success", "已重命名"));
      } else {
        showToast(t("kb_not_found", "知识库不存在"));
        setKbMissing(true);
      }
    } catch {
      showToast(t("rename_failed", "重命名失败，请重试"));
    } finally {
      setRenameOpen(false);
    }
  };

  const handleDeleteKbConfirm = async () => {
    if (!knowledgeBaseId) {
      return;
    }
    try {
      await KnowledgeBaseService.deleteKnowledgeBase(knowledgeBaseId);
      showToast(t("delete_success", "已删除"));
      navigate("/workspace/knowledge", { replace: true });
    } catch {
      showToast(t("delete_failed", "删除失败，请重试"));
    } finally {
      setDeleteKbOpen(false);
    }
  };

  const handleDeleteDocConfirm = async () => {
    if (!deleteDoc) {
      return;
    }
    try {
      await KnowledgeBaseService.deleteDocument(deleteDoc.id);
      showToast(t("doc_delete_success", "已删除"));
      setDeleteDoc(null);
      setMenuDoc(null);
      await load();
    } catch {
      showToast(t("doc_delete_failed", "删除失败，请重试"));
    }
  };

  if (kbMissing) {
    return (
      <PageLayout title={t("detail_title", "知识库详情")}>
        <div className="flex flex-col items-center justify-center flex-1 px-6 text-center">
          <p className="text-[15px] text-text-sub mb-4">
            {t("kb_not_found", "知识库不存在或已被删除")}
          </p>
          <button
            onClick={() => navigate("/workspace/knowledge", { replace: true })}
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
      title={kb?.name ?? t("detail_title", "知识库详情")}
      rightElement={
        <button
          onClick={() => setKbMenuOpen(true)}
          className="p-2 rounded-full hover:bg-black/5 dark:hover:bg-white/5 active:scale-95 transition-all"
          aria-label={t("menu_more", "更多操作")}
        >
          <MoreHorizontal className="w-6 h-6 text-text-main" strokeWidth={2.5} />
        </button>
      }
    >
      {kb?.description && (
        <div className="px-4 pt-3 pb-1">
          <p className="text-[13px] text-text-sub leading-relaxed line-clamp-2">
            {kb.description}
          </p>
        </div>
      )}

      <div className="flex items-center justify-between px-4 pt-4 pb-2">
        <div className="flex items-center gap-2">
          <span className="text-[16px] font-semibold text-text-main">
            {t("documents", "文档")}
          </span>
          <span className="text-[12px] text-text-sub bg-gray-100 dark:bg-gray-800 px-2 py-0.5 rounded-full">
            {documents.length}
          </span>
        </div>
        <button
          onClick={() => navigate(`/workspace/knowledge/${knowledgeBaseId}/doc/create`)}
          className="flex items-center gap-1.5 px-3.5 py-2 rounded-xl bg-primary-blue text-white text-[13px] font-medium active:scale-95 transition-transform"
        >
          <FilePlus2 className="w-4 h-4" />
          {t("create_doc", "新建文档")}
        </button>
      </div>

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
      ) : (
        <>
          {documents.length > 0 && (
            <div className="px-4 pb-3">
              <div className="flex items-center bg-[#f3f4f6] dark:bg-[#2c2d2e] rounded-xl px-4 py-2.5 transition-colors">
                <input
                  type="text"
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  placeholder={t("search_docs", "搜索文档...")}
                  className="bg-transparent border-none outline-none flex-1 text-[15px] text-text-main"
                />
              </div>
            </div>
          )}

          {filteredDocuments.length === 0 ? (
            <div className="flex flex-col items-center justify-center py-16 px-4 text-center">
              <div className="w-14 h-14 rounded-2xl bg-primary-blue/10 dark:bg-primary-blue/20 text-primary-blue flex items-center justify-center mb-3">
                <FilePlus2 className="w-7 h-7 text-primary-blue" />
              </div>
              <h3 className="text-[16px] font-semibold text-text-main mb-1">
                {t("no_docs", "暂无文档")}
              </h3>
              <p className="text-[13px] text-text-sub opacity-80 max-w-[260px] mb-5">
                {t("no_docs_desc", "该知识库还没有文档，创建第一篇文档开始沉淀知识。")}
              </p>
              <button
                onClick={() =>
                  navigate(`/workspace/knowledge/${knowledgeBaseId}/doc/create`)
                }
                className="px-5 py-2.5 rounded-xl bg-primary-blue text-white text-[14px] font-medium active:scale-95 transition-transform"
              >
                {t("create_doc", "新建文档")}
              </button>
            </div>
          ) : (
            <div className="flex flex-col gap-3 p-4 pt-1">
              {filteredDocuments.map((doc) => (
                <KnowledgeDocumentCard
                  key={doc.id}
                  doc={doc}
                  onClick={() =>
                    navigate(`/workspace/knowledge/${knowledgeBaseId}/doc/${doc.id}`)
                  }
                  onMoreClick={() => setMenuDoc(doc)}
                />
              ))}
            </div>
          )}
        </>
      )}

      <ActionSheet
        isOpen={kbMenuOpen}
        onClose={() => setKbMenuOpen(false)}
        title={kb?.name}
        options={[
          {
            label: t("menu_rename", "重命名"),
            onClick: () => setRenameOpen(true),
          },
          {
            label: t("menu_delete", "删除"),
            danger: true,
            onClick: () => setDeleteKbOpen(true),
          },
        ]}
      />

      <ActionSheet
        isOpen={menuDoc !== null}
        onClose={() => setMenuDoc(null)}
        title={menuDoc?.title}
        options={[
          {
            label: t("menu_open", "打开"),
            onClick: () =>
              menuDoc &&
              navigate(`/workspace/knowledge/${knowledgeBaseId}/doc/${menuDoc.id}`),
          },
          {
            label: t("menu_delete", "删除"),
            danger: true,
            onClick: () => menuDoc && setDeleteDoc(menuDoc),
          },
        ]}
      />

      <RenameDialog
        isOpen={renameOpen}
        title={t("rename_kb_title", "重命名知识库")}
        initialValue={kb?.name ?? ""}
        placeholder={t("name_placeholder", "知识库名称")}
        confirmText={t("save", "保存")}
        onConfirm={(value) => void handleRenameConfirm(value)}
        onCancel={() => setRenameOpen(false)}
      />

      <ConfirmDialog
        isOpen={deleteKbOpen}
        title={t("delete_kb_title", "删除知识库")}
        message={t("delete_kb_confirm", "确定删除该知识库及其全部文档吗？此操作不可恢复。")}
        confirmText={t("confirm_delete", "删除")}
        onConfirm={() => void handleDeleteKbConfirm()}
        onCancel={() => setDeleteKbOpen(false)}
      />

      <ConfirmDialog
        isOpen={deleteDoc !== null}
        title={t("doc_delete_title", "删除文档")}
        message={t("doc_delete_confirm", "确定删除该文档吗？此操作不可恢复。")}
        confirmText={t("confirm_delete", "删除")}
        onConfirm={() => void handleDeleteDocConfirm()}
        onCancel={() => setDeleteDoc(null)}
      />
    </PageLayout>
  );
}
