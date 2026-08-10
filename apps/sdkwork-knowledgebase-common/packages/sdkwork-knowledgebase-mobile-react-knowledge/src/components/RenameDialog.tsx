import React, { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { motion } from "motion/react";
import { createPortal } from "react-dom";

interface RenameDialogProps {
  isOpen: boolean;
  title: string;
  initialValue: string;
  placeholder?: string;
  confirmText?: string;
  cancelText?: string;
  onConfirm: (value: string) => void;
  onCancel: () => void;
}

export const RenameDialog: React.FC<RenameDialogProps> = ({
  isOpen,
  title,
  initialValue,
  placeholder,
  confirmText,
  cancelText,
  onConfirm,
  onCancel,
}) => {
  const { t } = useTranslation();
  const [value, setValue] = useState(initialValue);

  useEffect(() => {
    if (isOpen) {
      setValue(initialValue);
    }
  }, [isOpen, initialValue]);

  if (!isOpen) return null;

  const handleConfirm = () => {
    const trimmed = value.trim();
    if (trimmed.length === 0) {
      return;
    }
    onConfirm(trimmed);
  };

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
        <h3 className="text-[17px] font-semibold text-text-main text-center mb-4">{title}</h3>
        <input
          type="text"
          value={value}
          onChange={(e) => setValue(e.target.value)}
          placeholder={placeholder}
          autoFocus
          className="w-full px-4 py-3 rounded-xl bg-[#f3f4f6] dark:bg-[#2c2d2e] border border-transparent focus:border-primary-blue outline-none text-[15px] text-text-main mb-6 transition-colors"
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              handleConfirm();
            }
          }}
        />
        <div className="flex items-center gap-3">
          <button
            onClick={onCancel}
            className="flex-1 py-3 rounded-xl bg-gray-100 dark:bg-[#2c2d2e] text-text-main text-[15px] font-medium active:scale-95 transition-transform"
          >
            {cancelText ?? t("knowledge.cancel", "取消")}
          </button>
          <button
            onClick={handleConfirm}
            disabled={value.trim().length === 0}
            className="flex-1 py-3 rounded-xl bg-primary-blue text-white text-[15px] font-medium active:scale-95 transition-transform disabled:opacity-40 disabled:active:scale-100"
          >
            {confirmText ?? t("knowledge.save", "保存")}
          </button>
        </div>
      </motion.div>
    </div>,
    document.body,
  );
};
