import React from "react";
import { motion } from "motion/react";
import { MoreHorizontal, CheckCircle2, Circle, Archive } from "lucide-react";
import { useTranslation } from "react-i18next";
import { KnowledgeBase } from "../services/KnowledgeBaseService";

interface KnowledgeBaseCardProps {
  kb: KnowledgeBase;
  isSelectionMode: boolean;
  isSelected: boolean;
  onToggleSelect: () => void;
  onClickCard: () => void;
  onMoreClick: (e: React.MouseEvent) => void;
}

export const KnowledgeBaseCard: React.FC<KnowledgeBaseCardProps> = ({
  kb,
  isSelectionMode,
  isSelected,
  onToggleSelect,
  onClickCard,
  onMoreClick,
}) => {
  const { t } = useTranslation("knowledge");

  return (
    <motion.div
      whileHover={{ scale: 1.01 }}
      whileTap={{ scale: 0.98 }}
      onClick={() => {
        if (isSelectionMode) {
          onToggleSelect();
        } else {
          onClickCard();
        }
      }}
      className={`bg-white dark:bg-[#1e1e20] p-5 rounded-2xl shadow-sm border transition-all cursor-pointer flex flex-col relative ${
        isSelected
          ? "border-primary-blue ring-2 ring-primary-blue/20 bg-primary-blue/5 dark:bg-primary-blue/10"
          : "border-border-color/50 hover:shadow-md"
      }`}
    >
      <div className="flex items-start justify-between mb-3">
        <div className="flex items-center gap-3">
          {isSelectionMode && (
            <div className="shrink-0 transition-transform active:scale-95">
              {isSelected ? (
                <CheckCircle2 className="w-6 h-6 text-primary-blue fill-primary-blue/20" />
              ) : (
                <Circle className="w-6 h-6 text-text-sub opacity-50" />
              )}
            </div>
          )}
          <div
            className="w-12 h-12 rounded-xl flex items-center justify-center text-2xl shadow-inner shrink-0"
            style={{
              backgroundColor: kb.color ? `${kb.color}1A` : "rgba(0, 102, 255, 0.1)",
              color: kb.color || "#0066FF",
            }}
          >
            {kb.icon || "📚"}
          </div>
        </div>

        {!isSelectionMode && (
          <button
            className="p-1.5 rounded-full hover:bg-black/5 dark:hover:bg-white/5 text-text-sub transition-colors"
            onClick={(e) => {
              e.stopPropagation();
              onMoreClick(e);
            }}
          >
            <MoreHorizontal className="w-5 h-5" />
          </button>
        )}
      </div>

      <div className="flex items-center gap-2 mb-1">
        <h3 className="text-[17px] font-semibold text-text-main line-clamp-1 flex-1">
          {kb.name}
        </h3>
        {kb.isArchived && (
          <span className="shrink-0 flex items-center gap-1 text-[11px] px-2 py-0.5 rounded-full bg-amber-500/10 text-amber-600 dark:text-amber-400 font-medium border border-amber-500/20">
            <Archive className="w-3 h-3" />
            {t("archived", "已归档")}
          </span>
        )}
      </div>

      <p className="text-[14px] text-text-sub line-clamp-2 mb-4 h-10">
        {kb.description || t("no_description", "暂无描述")}
      </p>

      <div className="flex items-center justify-between mt-auto pt-3 border-t border-border-color/50">
        <span className="text-[12px] text-text-sub font-medium bg-gray-100 dark:bg-gray-800 px-2 py-1 rounded-md">
          {kb.createdAt
            ? new Date(kb.createdAt).toLocaleDateString()
            : ""}
        </span>
      </div>
    </motion.div>
  );
};
