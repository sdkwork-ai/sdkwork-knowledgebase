import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router";
import { PageLayout, showToast } from "@sdkwork/ui-mobile-react";

import { KnowledgeBaseService } from "../services/KnowledgeBaseService";

const KB_ICONS = ["📚", "🗂️", "📖", "🧠", "💡", "📝", "🚀", "🎯", "🏢", "🌱", "🔬", "🎓"];

const KB_COLORS = [
  "#0066FF",
  "#10B981",
  "#F59E0B",
  "#EF4444",
  "#8B5CF6",
  "#EC4899",
  "#14B8A6",
  "#6366F1",
];

export function CreateKnowledgeBase() {
  const { t } = useTranslation("knowledge");
  const navigate = useNavigate();

  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [icon, setIcon] = useState(KB_ICONS[0]);
  const [color, setColor] = useState(KB_COLORS[0]);
  const [submitting, setSubmitting] = useState(false);

  const handleSubmit = async () => {
    const trimmedName = name.trim();
    if (trimmedName.length === 0) {
      showToast(t("name_required", "请输入知识库名称"));
      return;
    }
    setSubmitting(true);
    try {
      await KnowledgeBaseService.createKnowledgeBase({
        name: trimmedName,
        description: description.trim(),
        icon,
        color,
      });
      showToast(t("create_success", "创建成功"));
      navigate("/workspace/knowledge", { replace: true });
    } catch {
      showToast(t("create_failed", "创建失败，请重试"));
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <PageLayout title={t("create_kb", "新建知识库")}>
      <div className="flex-1 overflow-y-auto p-4 pb-8">
        <div className="flex items-center gap-4 mb-6">
          <div
            className="w-16 h-16 rounded-2xl flex items-center justify-center text-3xl shadow-inner shrink-0"
            style={{
              backgroundColor: `${color}1A`,
              color,
            }}
          >
            {icon}
          </div>
          <div className="flex-1">
            <p className="text-[15px] font-semibold text-text-main mb-1">
              {t("icon", "图标")}
            </p>
            <div className="flex flex-wrap gap-2">
              {KB_ICONS.map((item) => (
                <button
                  key={item}
                  onClick={() => setIcon(item)}
                  className={`w-9 h-9 rounded-lg flex items-center justify-center text-[18px] transition-all active:scale-90 ${
                    icon === item
                      ? "bg-primary-blue/10 ring-2 ring-primary-blue"
                      : "bg-[#f3f4f6] dark:bg-[#2c2d2e]"
                  }`}
                >
                  {item}
                </button>
              ))}
            </div>
          </div>
        </div>

        <div className="mb-6">
          <p className="text-[13px] text-text-sub font-medium mb-2">{t("color", "颜色")}</p>
          <div className="flex flex-wrap gap-2.5">
            {KB_COLORS.map((item) => (
              <button
                key={item}
                onClick={() => setColor(item)}
                className={`w-9 h-9 rounded-full transition-all active:scale-90 ${
                  color === item
                    ? "ring-2 ring-offset-2 ring-offset-white dark:ring-offset-[#1e1e20] ring-[#0066FF]"
                    : ""
                }`}
                style={{ backgroundColor: item }}
                aria-label={item}
              />
            ))}
          </div>
        </div>

        <label className="block mb-4">
          <span className="text-[13px] text-text-sub font-medium mb-2 block">
            {t("kb_name", "知识库名称")} <span className="text-red-500">*</span>
          </span>
          <input
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder={t("name_placeholder", "例如：员工手册")}
            maxLength={60}
            className="w-full px-4 py-3 rounded-xl bg-[#f3f4f6] dark:bg-[#2c2d2e] border border-transparent focus:border-primary-blue outline-none text-[15px] text-text-main transition-colors"
          />
        </label>

        <label className="block mb-8">
          <span className="text-[13px] text-text-sub font-medium mb-2 block">
            {t("kb_desc", "描述")}
          </span>
          <textarea
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            placeholder={t("desc_placeholder", "简要描述这个知识库的用途...")}
            maxLength={200}
            rows={4}
            className="w-full px-4 py-3 rounded-xl bg-[#f3f4f6] dark:bg-[#2c2d2e] border border-transparent focus:border-primary-blue outline-none text-[15px] text-text-main resize-none transition-colors"
          />
        </label>

        <button
          onClick={() => void handleSubmit()}
          disabled={submitting}
          className="w-full py-3.5 rounded-xl bg-primary-blue text-white text-[16px] font-semibold shadow-md shadow-blue-500/25 active:scale-[0.98] transition-all disabled:opacity-50"
        >
          {submitting ? t("creating", "创建中...") : t("create_kb_submit", "创建")}
        </button>
      </div>
    </PageLayout>
  );
}
