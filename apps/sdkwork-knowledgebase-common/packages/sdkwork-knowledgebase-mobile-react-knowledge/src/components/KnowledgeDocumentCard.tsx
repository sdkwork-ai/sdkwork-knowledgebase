import React from "react";
import { motion } from "motion/react";
import { ChevronRight, MoreHorizontal } from "lucide-react";
import { useTranslation } from "react-i18next";
import { KnowledgeDocument } from "../services/KnowledgeBaseService";

interface KnowledgeDocumentCardProps {
  doc: KnowledgeDocument;
  onClick: () => void;
  onMoreClick?: (e: React.MouseEvent) => void;
}

export const KnowledgeDocumentCard: React.FC<KnowledgeDocumentCardProps> = ({
  doc,
  onClick,
  onMoreClick,
}) => {
  const { t } = useTranslation("knowledge");

  return (
    <motion.div
      whileHover={{ scale: 1.01 }}
      whileTap={{ scale: 0.98 }}
      onClick={onClick}
      className="bg-white dark:bg-[#1e1e20] p-4 rounded-xl shadow-sm border border-border-color/50 flex flex-col cursor-pointer hover:shadow-md transition-all"
    >
      <div className="flex justify-between items-start mb-2">
        <h3 className="text-[16px] font-semibold text-text-main leading-tight line-clamp-1 pr-4">
          {doc.title}
        </h3>
        <div className="flex items-center gap-1 shrink-0">
          {onMoreClick && (
            <button
              className="p-1.5 rounded-full hover:bg-black/5 dark:hover:bg-white/5 text-text-sub transition-colors"
              onClick={(e) => {
                e.stopPropagation();
                onMoreClick(e);
              }}
              aria-label={t("menu_more", "更多操作")}
            >
              <MoreHorizontal className="w-[18px] h-[18px]" />
            </button>
          )}
          <ChevronRight className="w-5 h-5 text-text-sub shrink-0 opacity-50" />
        </div>
      </div>
      {doc.content && doc.content.trim().length > 0 && (
        <p className="text-[14px] text-text-sub line-clamp-2 mb-3">
          {doc.content}
        </p>
      )}
      <div className="flex items-center justify-between mt-auto pt-3 border-t border-border-color/50">
        {doc.category ? (
          <span className="text-[12px] px-2.5 py-1 bg-primary-blue/10 text-primary-blue rounded-md font-medium">
            {doc.category}
          </span>
        ) : (
          <span />
        )}
        <div className="flex items-center gap-2">
          {doc.author && (
            <>
              <span className="text-[12px] text-text-sub">{doc.author}</span>
              <span className="text-[12px] text-text-sub/40">•</span>
            </>
          )}
          <span className="text-[12px] text-text-sub">
            {doc.createdAt
              ? new Date(doc.createdAt).toLocaleDateString()
              : ""}
          </span>
        </div>
      </div>
    </motion.div>
  );
};
