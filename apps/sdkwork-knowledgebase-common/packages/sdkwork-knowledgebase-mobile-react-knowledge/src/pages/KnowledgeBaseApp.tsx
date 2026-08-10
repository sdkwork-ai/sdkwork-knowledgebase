import React, { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";
import { Plus, RefreshCw } from "lucide-react";
import { ActionSheet, PageLayout, showToast } from "@sdkwork/ui-mobile-react";

import {
  KnowledgeBase,
  KnowledgeBaseService,
} from "../services/KnowledgeBaseService";
import { KnowledgeBaseCard } from "../components/KnowledgeBaseCard";
import { KnowledgeBaseHeaderFilter } from "../components/KnowledgeBaseHeaderFilter";
import { EmptyKnowledgeBaseState } from "../components/EmptyKnowledgeBaseState";
import { BatchActionBar } from "../components/BatchActionBar";
import { ConfirmDialog } from "../components/ConfirmDialog";
import { RenameDialog } from "../components/RenameDialog";

export function KnowledgeBaseApp() {
  const { t } = useTranslation("knowledge");
  const navigate = useNavigate();

  const [knowledgeBases, setKnowledgeBases] = useState<KnowledgeBase[]>([]);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState(false);

  const [searchQuery, setSearchQuery] = useState("");
  const [activeFilter, setActiveFilter] = useState("all");
  const [isSelectionMode, setIsSelectionMode] = useState(false);
  const [selectedIds, setSelectedIds] = useState<string[]>([]);

  const [menuKb, setMenuKb] = useState<KnowledgeBase | null>(null);
  const [renameKb, setRenameKb] = useState<KnowledgeBase | null>(null);
  const [deleteKb, setDeleteKb] = useState<KnowledgeBase | null>(null);
  const [batchDeleteOpen, setBatchDeleteOpen] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    setLoadError(false);
    try {
      setKnowledgeBases(await KnowledgeBaseService.getKnowledgeBases());
    } catch {
      setLoadError(true);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const filteredKnowledgeBases = useMemo(() => {
    const query = searchQuery.trim().toLowerCase();
    let list = knowledgeBases;
    if (query.length > 0) {
      list = list.filter(
        (kb) =>
          kb.name.toLowerCase().includes(query) ||
          kb.description.toLowerCase().includes(query),
      );
    }
    const sorted = [...list];
    switch (activeFilter) {
      case "newest":
        sorted.sort((a, b) => b.createdAt.localeCompare(a.createdAt));
        break;
      case "oldest":
        sorted.sort((a, b) => a.createdAt.localeCompare(b.createdAt));
        break;
      case "recently_updated":
        sorted.sort((a, b) => b.updatedAt.localeCompare(a.updatedAt));
        break;
      default:
        break;
    }
    return sorted;
  }, [knowledgeBases, searchQuery, activeFilter]);

  const toggleSelect = (id: string) => {
    setSelectedIds((prev) =>
      prev.includes(id) ? prev.filter((item) => item !== id) : [...prev, id],
    );
  };

  const handleRenameConfirm = async (value: string) => {
    if (!renameKb) {
      return;
    }
    try {
      const updated = await KnowledgeBaseService.updateKnowledgeBase(renameKb.id, {
        name: value,
      });
      if (updated) {
        showToast(t("rename_success", "已重命名"));
        setRenameKb(null);
        await load();
      } else {
        showToast(t("kb_not_found", "知识库不存在"));
        setRenameKb(null);
      }
    } catch {
      showToast(t("rename_failed", "重命名失败，请重试"));
    } finally {
      }
  };

  const handleDeleteConfirm = async () => {
    if (!deleteKb) {
      return;
    }
    try {
      await KnowledgeBaseService.deleteKnowledgeBase(deleteKb.id);
      showToast(t("delete_success", "已删除"));
      setDeleteKb(null);
      setMenuKb(null);
      setSelectedIds((prev) => prev.filter((id) => id !== deleteKb.id));
      await load();
    } catch {
      showToast(t("delete_failed", "删除失败，请重试"));
    } finally {
      }
  };

  const handleBatchDeleteConfirm = async () => {
    try {
      await KnowledgeBaseService.deleteKnowledgeBases(selectedIds);
      showToast(t("delete_success", "已删除"));
      setBatchDeleteOpen(false);
      setIsSelectionMode(false);
      setSelectedIds([]);
      await load();
    } catch {
      showToast(t("delete_failed", "删除失败，请重试"));
    } finally {
      }
  };

  const cardActions = menuKb
    ? [
        {
          label: t("menu_open", "打开"),
          onClick: () => navigate(`/workspace/knowledge/${menuKb.id}`),
        },
        {
          label: t("menu_rename", "重命名"),
          onClick: () => setRenameKb(menuKb),
        },
        {
          label: t("menu_delete", "删除"),
          danger: true,
          onClick: () => setDeleteKb(menuKb),
        },
      ]
    : [];

  return (
    <PageLayout
      title={t("title")}
      rightElement={
        <button
          onClick={() => navigate("/workspace/knowledge/create")}
          className="p-2 rounded-full hover:bg-black/5 dark:hover:bg-white/5 active:scale-95 transition-all"
          aria-label={t("create_new", "新建知识库")}
        >
          <Plus className="w-6 h-6 text-primary-blue" strokeWidth={2.5} />
        </button>
      }
    >
      <KnowledgeBaseHeaderFilter
        searchQuery={searchQuery}
        setSearchQuery={setSearchQuery}
        activeFilter={activeFilter}
        setActiveFilter={setActiveFilter}
        isSelectionMode={isSelectionMode}
        setIsSelectionMode={(value) => {
          setIsSelectionMode(value);
          if (!value) {
            setSelectedIds([]);
          }
        }}
        totalCount={knowledgeBases.length}
      />

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
      ) : filteredKnowledgeBases.length === 0 ? (
        <EmptyKnowledgeBaseState
          onCreateNew={() => navigate("/workspace/knowledge/create")}
        />
      ) : (
        <div className="grid grid-cols-2 gap-3 p-4">
          {filteredKnowledgeBases.map((kb) => (
            <KnowledgeBaseCard
              key={kb.id}
              kb={kb}
              isSelectionMode={isSelectionMode}
              isSelected={selectedIds.includes(kb.id)}
              onToggleSelect={() => toggleSelect(kb.id)}
              onClickCard={() => navigate(`/workspace/knowledge/${kb.id}`)}
              onMoreClick={() => setMenuKb(kb)}
            />
          ))}
        </div>
      )}

      {isSelectionMode && (
        <BatchActionBar
          selectedCount={selectedIds.length}
          totalCount={filteredKnowledgeBases.length}
          isAllSelected={
            filteredKnowledgeBases.length > 0 &&
            selectedIds.length === filteredKnowledgeBases.length
          }
          onToggleSelectAll={() => {
            const allIds = filteredKnowledgeBases.map((kb) => kb.id);
            setSelectedIds(
              selectedIds.length === allIds.length ? [] : allIds,
            );
          }}
          onBatchDelete={() => setBatchDeleteOpen(true)}
          onCancel={() => {
            setIsSelectionMode(false);
            setSelectedIds([]);
          }}
        />
      )}

      <ActionSheet
        isOpen={menuKb !== null}
        onClose={() => setMenuKb(null)}
        title={menuKb?.name}
        options={cardActions}
      />

      <RenameDialog
        isOpen={renameKb !== null}
        title={t("rename_kb_title", "重命名知识库")}
        initialValue={renameKb?.name ?? ""}
        placeholder={t("name_placeholder", "知识库名称")}
        confirmText={t("save", "保存")}
        onConfirm={(value) => void handleRenameConfirm(value)}
        onCancel={() => setRenameKb(null)}
      />

      <ConfirmDialog
        isOpen={deleteKb !== null}
        title={t("delete_kb_title", "删除知识库")}
        message={t("delete_kb_confirm", "确定删除该知识库及其全部文档吗？此操作不可恢复。")}
        confirmText={t("confirm_delete", "删除")}
        onConfirm={() => void handleDeleteConfirm()}
        onCancel={() => setDeleteKb(null)}
      />

      <ConfirmDialog
        isOpen={batchDeleteOpen}
        title={t("batch_delete_title", "批量删除")}
        message={t(
          "batch_delete_confirm",
          "确定删除选中的 {{count}} 个知识库吗？此操作不可恢复。",
          { count: selectedIds.length },
        )}
        confirmText={t("confirm_delete", "删除")}
        onConfirm={() => void handleBatchDeleteConfirm()}
        onCancel={() => setBatchDeleteOpen(false)}
      />
    </PageLayout>
  );
}
