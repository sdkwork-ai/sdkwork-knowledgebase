import type { Dispatch, SetStateAction } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { CreateKbModal } from './CreateKbModal';
import { PublishModal } from './PublishModal';
import { KnowledgeBaseSettingsModal } from './KnowledgeBaseSettingsModal';
import { KnowledgeBaseMarketModal } from './KnowledgeBaseMarketModal';
import { CloudDriveModal } from './CloudDriveModal';
import { GitIntegrationModal } from './components/GitIntegrationModal';
import { DocumentService, type DocumentMeta, type KnowledgeBase } from './services/document';
import type { CloudDriveImportResultItem } from './services/cloudDriveService';
import { toastKnowledgebaseError } from './components/ui/toastKnowledgebaseError';

export interface KnowledgeBaseAppModalsProps {
  isCreateKbModalOpen: boolean;
  setIsCreateKbModalOpen: (open: boolean) => void;
  newKbTitle: string;
  setNewKbTitle: (title: string) => void;
  newKbType: 'team' | 'personal' | 'public';
  setNewKbType: (type: 'team' | 'personal' | 'public') => void;
  newKbIcon: string;
  setNewKbIcon: (icon: string) => void;
  onCreateKb: () => void | Promise<void>;
  isPublishModalOpen: boolean;
  setIsPublishModalOpen: (open: boolean) => void;
  publishDocsContext: DocumentMeta[];
  setPublishDocsContext: Dispatch<SetStateAction<DocumentMeta[]>>;
  settingsKb: KnowledgeBase | null;
  setSettingsKb: (kb: KnowledgeBase | null) => void;
  activeKb: KnowledgeBase | null;
  setActiveKb: Dispatch<SetStateAction<KnowledgeBase | null>>;
  setActiveDoc: Dispatch<SetStateAction<DocumentMeta | null>>;
  setKbs: Dispatch<SetStateAction<{ team: KnowledgeBase[]; personal: KnowledgeBase[]; public: KnowledgeBase[] }>>;
  isMarketOpen: boolean;
  setIsMarketOpen: (open: boolean) => void;
  gitModalKb: KnowledgeBase | null;
  setGitModalKb: (kb: KnowledgeBase | null) => void;
  gitModalMode: 'import' | 'sync' | null;
  setGitModalMode: (mode: 'import' | 'sync' | null) => void;
  setLoadingDocs: (loading: boolean) => void;
  setDocs: Dispatch<SetStateAction<any[]>>;
  cloudDriveKb: KnowledgeBase | null;
  setCloudDriveKb: (kb: KnowledgeBase | null) => void;
  onImportCloudDrive: (selectedItems: CloudDriveImportResultItem[]) => Promise<void>;
}

export function KnowledgeBaseAppModals(props: KnowledgeBaseAppModalsProps) {
  const navigate = useNavigate();
  const { t } = useTranslation(['kb', 'common', 'errors']);
  const {
    isCreateKbModalOpen,
    setIsCreateKbModalOpen,
    newKbTitle,
    setNewKbTitle,
    newKbType,
    setNewKbType,
    newKbIcon,
    setNewKbIcon,
    onCreateKb,
    isPublishModalOpen,
    setIsPublishModalOpen,
    publishDocsContext,
    setPublishDocsContext,
    settingsKb,
    setSettingsKb,
    activeKb,
    setActiveKb,
    setActiveDoc,
    setKbs,
    isMarketOpen,
    setIsMarketOpen,
    gitModalKb,
    setGitModalKb,
    gitModalMode,
    setGitModalMode,
    setLoadingDocs,
    setDocs,
    cloudDriveKb,
    setCloudDriveKb,
    onImportCloudDrive,
  } = props;

  return (
    <>
      {isCreateKbModalOpen && (
        <CreateKbModal
          newKbTitle={newKbTitle}
          setNewKbTitle={setNewKbTitle}
          newKbType={newKbType}
          setNewKbType={setNewKbType}
          newKbIcon={newKbIcon}
          setNewKbIcon={setNewKbIcon}
          onCancel={() => setIsCreateKbModalOpen(false)}
          onCreate={onCreateKb}
        />
      )}

      {isPublishModalOpen && (
        <PublishModal
          documents={publishDocsContext}
          onClose={() => {
            setIsPublishModalOpen(false);
            setPublishDocsContext([]);
          }}
          onWechatFlow={() => {
            setIsPublishModalOpen(false);
            navigate('/wechat-publish', { state: { documents: publishDocsContext } });
          }}
        />
      )}

      {settingsKb && (
        <KnowledgeBaseSettingsModal
          kb={settingsKb}
          onClose={() => setSettingsKb(null)}
          onSave={async (updates) => {
            try {
              const updated = await DocumentService.updateKnowledgeBase(settingsKb.id, updates);
              const data = await DocumentService.getKnowledgeBases();
              setKbs(data);
              if (activeKb && activeKb.id === settingsKb.id) {
                setActiveKb(updated);
              }
              setSettingsKb(null);
            } catch (error) {
              console.error(error);
              toastKnowledgebaseError(error, t);
            }
          }}
        />
      )}

      {isMarketOpen && (
        <KnowledgeBaseMarketModal
          onClose={() => setIsMarketOpen(false)}
          onSubscribedChange={() => {
            DocumentService.getKnowledgeBases().then((data) => {
              setKbs(data);
              if (activeKb) {
                const stillExists = [
                  ...(data.team || []),
                  ...(data.personal || []),
                  ...(data.public || []),
                ].find((kb) => kb.id === activeKb.id);
                if (!stillExists) {
                  setActiveKb(null);
                  setDocs([]);
                  setActiveDoc(null);
                }
              }
            });
          }}
        />
      )}

      {gitModalKb && gitModalMode && (
        <GitIntegrationModal
          mode={gitModalMode}
          kb={gitModalKb}
          onClose={() => {
            setGitModalKb(null);
            setGitModalMode(null);
          }}
          onSuccess={() => {
            if (activeKb && activeKb.id === gitModalKb.id) {
              setLoadingDocs(true);
              DocumentService.getDocuments(activeKb.id).then((data) => {
                setDocs(data);
                setLoadingDocs(false);
              });
            }
          }}
        />
      )}

      {cloudDriveKb && (
        <CloudDriveModal
          isOpen={!!cloudDriveKb}
          onClose={() => setCloudDriveKb(null)}
          spaceId={cloudDriveKb.id}
          onConfirm={onImportCloudDrive}
        />
      )}
    </>
  );
}
