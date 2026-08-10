import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useParams } from "react-router";
import { PageLayout, showToast } from "@sdkwork/ui-mobile-react";

import { KnowledgeBaseService } from "../services/KnowledgeBaseService";

export function CreateDocument() {
  const { t } = useTranslation("knowledge");
  const navigate = useNavigate();
  const { id: knowledgeBaseId } = useParams<{ id: string }>();

  const [title, setTitle] = useState("");
  const [content, setContent] = useState("");
  const [submitting, setSubmitting] = useState(false);

  const handleSubmit = async () => {
    if (!knowledgeBaseId) {
      return;
    }
    const trimmedTitle = title.trim();
    if (trimmedTitle.length === 0) {
      showToast(t("doc_title_required", "请输入文档标题"));
      return;
    }
    setSubmitting(true);
    try {
      await KnowledgeBaseService.createDocument({
        kbId: knowledgeBaseId,
        title: trimmedTitle,
        content,
        category: "",
        author: "",
      });
      showToast(t("create_doc_success", "文档已创建，内容同步中"));
      navigate(`/workspace/knowledge/${knowledgeBaseId}`, { replace: true });
    } catch {
      showToast(t("create_doc_failed", "创建失败，请重试"));
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <PageLayout title={t("create_doc", "新建文档")}>
      <div className="flex-1 overflow-y-auto p-4 pb-8">
        <label className="block mb-4">
          <span className="text-[13px] text-text-sub font-medium mb-2 block">
            {t("doc_title", "文档标题")} <span className="text-red-500">*</span>
          </span>
          <input
            type="text"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            placeholder={t("doc_title_placeholder", "例如：季度产品复盘")}
            maxLength={80}
            className="w-full px-4 py-3 rounded-xl bg-[#f3f4f6] dark:bg-[#2c2d2e] border border-transparent focus:border-primary-blue outline-none text-[15px] text-text-main transition-colors"
          />
        </label>

        <label className="block mb-6">
          <span className="text-[13px] text-text-sub font-medium mb-2 block">
            {t("doc_content", "内容")}
          </span>
          <textarea
            value={content}
            onChange={(e) => setContent(e.target.value)}
            placeholder={t("content_placeholder", "开始输入内容，支持 Markdown...")}
            rows={14}
            className="w-full px-4 py-3 rounded-xl bg-[#f3f4f6] dark:bg-[#2c2d2e] border border-transparent focus:border-primary-blue outline-none text-[15px] text-text-main leading-relaxed resize-none transition-colors"
          />
        </label>

        <p className="text-[12px] text-text-sub/70 mb-6">
          {t("create_doc_hint", "文档创建后内容将通过异步索引管线同步，稍后刷新即可查看全文。")}
        </p>

        <button
          onClick={() => void handleSubmit()}
          disabled={submitting}
          className="w-full py-3.5 rounded-xl bg-primary-blue text-white text-[16px] font-semibold shadow-md shadow-blue-500/25 active:scale-[0.98] transition-all disabled:opacity-50"
        >
          {submitting ? t("creating", "创建中...") : t("create_doc_submit", "保存")}
        </button>
      </div>
    </PageLayout>
  );
}
