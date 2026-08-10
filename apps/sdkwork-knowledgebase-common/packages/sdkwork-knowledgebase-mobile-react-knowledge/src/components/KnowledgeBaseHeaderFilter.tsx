import React from "react";
import { Search, CheckSquare } from "lucide-react";
import { useTranslation } from "react-i18next";

interface KnowledgeBaseHeaderFilterProps {
  searchQuery: string;
  setSearchQuery: (val: string) => void;
  activeFilter: string;
  setActiveFilter: (val: string) => void;
  isSelectionMode: boolean;
  setIsSelectionMode: (val: boolean) => void;
  totalCount: number;
}

export const KnowledgeBaseHeaderFilter: React.FC<KnowledgeBaseHeaderFilterProps> = ({
  searchQuery,
  setSearchQuery,
  activeFilter,
  setActiveFilter,
  isSelectionMode,
  setIsSelectionMode,
  totalCount,
}) => {
  const { t } = useTranslation("knowledge");

  const filterOptions = [
    { id: "all", label: t("filter_all", "全部") },
    { id: "newest", label: t("filter_newest", "最新创建") },
    { id: "oldest", label: t("filter_oldest", "最早创建") },
    { id: "recently_updated", label: t("filter_recent", "最近活跃") },
  ];

  return (
    <div className="p-4 bg-white dark:bg-[#1e1e20] shadow-sm mb-4">
      <div className="flex items-center gap-2 mb-3">
        <div className="flex-1 flex items-center bg-[#f3f4f6] dark:bg-[#2c2d2e] rounded-xl px-4 py-2.5 transition-colors">
          <Search className="w-5 h-5 text-text-sub mr-2 shrink-0" />
          <input
            type="text"
            placeholder={t("search_kb", "搜索知识库...")}
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="bg-transparent border-none outline-none flex-1 text-[15px] text-text-main"
          />
        </div>

        <button
          onClick={() => setIsSelectionMode(!isSelectionMode)}
          disabled={totalCount === 0}
          className={`flex items-center gap-1.5 px-3 py-2.5 rounded-xl text-[13px] font-medium transition-all disabled:opacity-40 ${
            isSelectionMode
              ? "bg-primary-blue text-white shadow-sm active:scale-95"
              : "bg-[#f3f4f6] dark:bg-[#2c2d2e] text-text-main hover:bg-gray-200 dark:hover:bg-gray-700 active:scale-95"
          }`}
        >
          <CheckSquare className="w-4 h-4" />
          <span>
            {isSelectionMode
              ? t("exit_batch", "取消批量")
              : t("batch", "批量操作")}
          </span>
        </button>
      </div>

      {/* Quick Filters */}
      <div className="flex items-center gap-2 overflow-x-auto no-scrollbar pb-1">
        {filterOptions.map((filter) => (
          <button
            key={filter.id}
            onClick={() => setActiveFilter(filter.id)}
            className={`whitespace-nowrap px-4 py-1.5 rounded-full text-[13px] font-medium transition-colors ${
              activeFilter === filter.id
                ? "bg-primary-blue text-white"
                : "bg-[#f3f4f6] dark:bg-[#2c2d2e] text-text-sub hover:bg-gray-200 dark:hover:bg-gray-700"
            }`}
          >
            {filter.label}
          </button>
        ))}
      </div>
    </div>
  );
};
