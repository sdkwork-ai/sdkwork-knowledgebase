import React from "react";
import { useTranslation } from "react-i18next";
import { motion } from "motion/react";
import { createPortal } from "react-dom";

interface ConfirmDialogProps {
  isOpen: boolean;
  title: string;
  message?: string;
  confirmText?: string;
  cancelText?: string;
  danger?: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}

export const ConfirmDialog: React.FC<ConfirmDialogProps> = ({
  isOpen,
  title,
  message,
  confirmText,
  cancelText,
  danger = true,
  onConfirm,
  onCancel,
}) => {
  const { t } = useTranslation();

  if (!isOpen) return null;

  return createPortal(
    <div className="fixed inset-0 z-50 flex items-center justify-center px-6">
      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        className="absolute inset-0 bg-black/40"
        onClick={onCancel}
      />
      <motion.div
        initial={{ scale: 0.92, opacity: 0 }}
        animate={{ scale: 1, opacity: 1 }}
        transition={{ type: "spring", damping: 24, stiffness: 320 }}
        className="relative w-full max-w-[320px] bg-white dark:bg-[#1e1e20] rounded-2xl p-6 shadow-2xl"
      >
        <h3 className="text-[17px] font-semibold text-text-main text-center mb-2">{title}</h3>
        {message && (
          <p className="text-[14px] text-text-sub text-center leading-relaxed mb-6">{message}</p>
        )}
        <div className="flex items-center gap-3">
          <button
            onClick={onCancel}
            className="flex-1 py-3 rounded-xl bg-gray-100 dark:bg-[#2c2d2e] text-text-main text-[15px] font-medium active:scale-95 transition-transform"
          >
            {cancelText ?? t("knowledge.cancel", "取消")}
          </button>
          <button
            onClick={onConfirm}
            className={`flex-1 py-3 rounded-xl text-white text-[15px] font-medium active:scale-95 transition-transform ${
              danger
                ? "bg-red-500 hover:bg-red-600"
                : "bg-primary-blue hover:bg-blue-600"
            }`}
          >
            {confirmText ?? t("knowledge.confirm", "确认")}
          </button>
        </div>
      </motion.div>
    </div>,
    document.body,
  );
};
