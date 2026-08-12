import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useParams } from "react-router";
import { PageLayout, showToast } from "@sdkwork/ui-mobile-react";

import { KnowledgeBaseService } from "../services/KnowledgeBaseService";

import { CapabilityUnavailablePage } from "../components/CapabilityUnavailablePage";

export function CreateDocument() {
  return (
    <CapabilityUnavailablePage />
  );
}
