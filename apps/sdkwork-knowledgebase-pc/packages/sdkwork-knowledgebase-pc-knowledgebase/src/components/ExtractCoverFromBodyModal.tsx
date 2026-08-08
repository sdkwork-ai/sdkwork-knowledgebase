import { X, Check, Image as ImageIcon } from 'lucide-react';
import { useTranslation } from 'react-i18next';

export interface ExtractCoverFromBodyModalProps {
  isOpen: boolean;
  onClose: () => void;
  extractedBodyImages: string[];
  selectedBodyImage: string;
  setSelectedBodyImage: (src: string) => void;
  onConfirm: () => void;
}

export function ExtractCoverFromBodyModal({
  isOpen,
  onClose,
  extractedBodyImages,
  selectedBodyImage,
  setSelectedBodyImage,
  onConfirm
}: ExtractCoverFromBodyModalProps) {
  const { t } = useTranslation('common');

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black/50 z-[300] flex items-center justify-center p-4">
      <div className="bg-white w-[900px] h-[600px] rounded flex flex-col shadow-lg overflow-hidden">
        <div className="flex items-center justify-between p-6 pb-2">
          <span className="text-base font-normal text-gray-800">{t('chooseImage')}</span>
          <button 
            onClick={onClose} 
            className="text-gray-400 hover:text-gray-600"
          >
            <X size={20} strokeWidth={1} />
          </button>
        </div>
        
        <div className="px-6 py-2">
          <p className="text-sm text-gray-500 mb-6">{t('chooseImageHint')}</p>
        </div>

        <div className="px-6 overflow-y-auto flex-1">
          {extractedBodyImages.length > 0 ? (
            <div className="grid grid-cols-5 gap-4">
              {extractedBodyImages.map((src, i) => (
                <div 
                  key={i} 
                  className={`relative aspect-square bg-gray-100 cursor-pointer border-2 transition-all group ${
                    selectedBodyImage === src 
                      ? 'border-[#07c160]' 
                      : 'border-transparent hover:border-gray-300'
                  }`}
                  onClick={() => setSelectedBodyImage(src)}
                >
                  <img src={src} referrerPolicy="no-referrer" alt="extracted" className="w-full h-full object-cover" />
                  {selectedBodyImage === src && (
                    <div className="absolute top-1 right-1 bg-[#07c160] rounded-full p-0.5 text-white shadow">
                      <Check size={12} strokeWidth={3} />
                    </div>
                  )}
                </div>
              ))}
            </div>
          ) : (
            <div className="text-center py-20 flex flex-col items-center opacity-50">
              <ImageIcon size={48} className="text-gray-300 mb-4" />
              <p className="text-sm text-gray-500">{t('noImageInBody')}</p>
            </div>
          )}
        </div>
        
        <div className="flex justify-center items-center py-6 gap-4 bg-gray-50/50">
          <button 
            onClick={onConfirm} 
            disabled={!selectedBodyImage}
            className={`px-8 py-1.5 text-sm rounded transition-colors ${
              selectedBodyImage 
                ? 'bg-[#07c160] hover:bg-[#06ad56] text-white' 
                : 'bg-[#7ae2aa] text-white cursor-not-allowed opacity-[0.65]'
            }`}
          >
            {t('nextStep')}
          </button>
          <button 
            onClick={onClose} 
            className="px-8 py-1.5 text-sm bg-white border border-gray-200 hover:bg-gray-50 text-gray-600 rounded transition-colors"
          >
            {t('cancel')}
          </button>
        </div>
      </div>
    </div>
  );
}
