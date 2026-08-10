import React from "react";
import { Database, Plus } from "lucide-react";
import { motion } from "motion/react";
import { useTranslation } from "react-i18next";

interface EmptyKnowledgeBaseStateProps {
  onCreateNew: () => void;
}

export const EmptyKnowledgeBaseState: React.FC<EmptyKnowledgeBaseStateProps> = ({
  onCreateNew,
}) => {
  const { t } = useTranslation("knowledge");

  return (
    <div className="col-span-full flex flex-col items-center justify-center py-12 px-4 text-center">
      <div className="w-16 h-16 rounded-2xl bg-primary-blue/10 dark:bg-primary-blue/20 text-primary-blue flex items-center justify-center mb-4">
        <Database className="w-8 h-8 text-primary-blue Database" />
      </div>
      <h3 className="text-[17px] font-semibold text-text-main mb-1">
        {t("no_kbs", "暂无知识库")}
      </h3>
      <p className="text-[13px] text-text-sub opacity-80 max-w-[260px] mb-6">
        {t("no_kbs_desc", "当前暂无知识库，立即创建一个开始管理文档与知识。")}
      </p>

      <motion.button
        whileHover={{ scale: 1.03 }}
        whileTap={{ scale: 0.96 }}
        onClick={onCreateNew}
        className="px-6 py-3 rounded-xl bg-primary-blue text-white text-[15px] font-medium shadow-md shadow-blue-500/25 flex items-center gap-2 hover:bg-blue-600 transition-colors cursor-pointer"
      >
        <Plus className="w-5 h-5" />
        <span>{t("create_new", "新建知识库")}</span>
      </motion.button>
    </div>
  );
};
