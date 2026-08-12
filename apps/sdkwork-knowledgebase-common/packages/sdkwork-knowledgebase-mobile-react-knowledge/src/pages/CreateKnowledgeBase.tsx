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

import { CapabilityUnavailablePage } from "../components/CapabilityUnavailablePage";

export function CreateKnowledgeBase() {
  return (
    <CapabilityUnavailablePage />
  );
}
