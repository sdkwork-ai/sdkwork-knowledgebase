import { useMemo, useState } from 'react';
import {
  ExternalLink,
  ShieldCheck,
  ShoppingBag,
  ShoppingCart,
  Sparkles,
  Truck
} from 'lucide-react';
import { openExternalUrl } from './openExternalUrl';
import { MediaSourceChip } from './shared/MediaSourceChip';
import { StarRating } from './shared/StarRating';
import type { SearchMediaViewerContentProps } from './types';

function buildGallery(item: SearchMediaViewerContentProps['item']): string[] {
  const urls = [item.thumbnailUrl, ...(item.galleryUrls ?? [])].filter(Boolean) as string[];
  return urls.length > 0 ? Array.from(new Set(urls)) : [];
}

function parsePriceNumber(price?: string): number | null {
  if (!price) return null;
  const num = parseFloat(price.replace(/[^\d.]/g, ''));
  return Number.isFinite(num) && num > 0 ? num : null;
}

export function SearchProductViewerContent({ item, onOpenWebLink }: SearchMediaViewerContentProps) {
  const gallery = useMemo(() => buildGallery(item), [item]);
  const [activeImageIndex, setActiveImageIndex] = useState(0);
  const activeImage = gallery[activeImageIndex] ?? item.thumbnailUrl;
  const externalUrl = item.url ?? item.previewUrl;

  const highlights = item.highlights ?? [];
  const tags = item.tags ?? [];
  const specs = item.specs ?? [];
  const description =
    item.description ??
    item.snippet ??
    '该商品来自检索结果聚合，可在原平台查看完整参数、用户评价与购买选项。';

  const priceNum = parsePriceNumber(item.price);
  const originalNum = parsePriceNumber(item.originalPrice);
  const savingsPercent =
    priceNum && originalNum && originalNum > priceNum
      ? Math.round(((originalNum - priceNum) / originalNum) * 100)
      : null;

  const discount =
    savingsPercent != null ? `省 ${savingsPercent}%` : item.originalPrice && item.price && item.originalPrice !== item.price ? '限时优惠' : null;

  return (
    <div className="search-product-detail search-product-detail--pro">
      <div className="search-product-detail__gallery search-product-detail__gallery--pro">
        <div className="search-product-detail__gallery-row">
          {gallery.length > 1 && (
            <div className="search-product-detail__thumb-rail">
              {gallery.map((url, index) => (
                <button
                  key={`${url}-${index}`}
                  type="button"
                  className={`search-product-detail__thumb ${index === activeImageIndex ? 'search-product-detail__thumb--active' : ''}`}
                  onClick={() => setActiveImageIndex(index)}
                  aria-label={`查看图片 ${index + 1}`}
                >
                  <img src={url} alt={`${item.title} ${index + 1}`} loading="lazy" />
                </button>
              ))}
            </div>
          )}
          <div className="search-product-detail__hero-wrap">
            <div className="search-product-detail__hero search-product-detail__hero--pro">
              {activeImage ? (
                <img
                  key={activeImage}
                  src={activeImage}
                  alt={item.title}
                  className="search-product-detail__hero-image"
                />
              ) : (
                <div className="search-product-detail__hero-placeholder">
                  <ShoppingBag className="w-14 h-14 opacity-35" />
                </div>
              )}
            </div>
          </div>
        </div>
      </div>

      <div className="search-product-detail__panel search-product-detail__panel--pro">
        <div className="search-product-detail__scroll">
          <div className="search-product-detail__top">
            <MediaSourceChip item={item} />
            {item.merchant && <span className="search-product-detail__merchant">{item.merchant}</span>}
          </div>

          <h2 className="search-product-detail__title">{item.title}</h2>

          {tags.length > 0 && (
            <div className="search-product-detail__tags">
              {tags.map((tag) => (
                <span key={tag} className="search-product-detail__tag">
                  {tag}
                </span>
              ))}
            </div>
          )}

          <div className="search-product-detail__price-row">
            {item.price && <span className="search-product-detail__price">{item.price}</span>}
            {item.originalPrice && (
              <span className="search-product-detail__original-price">{item.originalPrice}</span>
            )}
            {savingsPercent != null && <span className="search-product-detail__savings">省 {savingsPercent}%</span>}
            {discount && savingsPercent == null && (
              <span className="search-product-detail__discount">{discount}</span>
            )}
          </div>

          {(item.rating != null || item.reviewCount != null) && (
            <div className="search-product-detail__rating-row">
              {item.rating != null && <StarRating rating={item.rating} />}
              {item.rating != null && (
                <span className="search-product-detail__rating-value">{item.rating.toFixed(1)}</span>
              )}
              {item.reviewCount != null && (
                <span className="search-product-detail__reviews">{item.reviewCount.toLocaleString()} 条评价</span>
              )}
            </div>
          )}

          <div className="search-product-detail__trust-strip">
            {item.shippingNote && (
              <span className="search-product-detail__service">
                <Truck className="w-3.5 h-3.5" />
                {item.shippingNote}
              </span>
            )}
            <span className="search-product-detail__service">
              <ShieldCheck className="w-3.5 h-3.5" />
              平台保障 · 正品溯源
            </span>
          </div>

          {highlights.length > 0 && (
            <section className="search-product-detail__section">
              <h3 className="search-product-detail__section-title">
                <Sparkles className="w-3.5 h-3.5" />
                核心亮点
              </h3>
              <ul className="search-product-detail__highlights">
                {highlights.map((point) => (
                  <li key={point} className="search-product-detail__highlight-card">
                    {point}
                  </li>
                ))}
              </ul>
            </section>
          )}

          {specs.length > 0 && (
            <section className="search-product-detail__section">
              <h3 className="search-product-detail__section-title">规格参数</h3>
              <dl className="search-product-detail__specs">
                {specs.map((spec) => (
                  <div key={spec.label} className="search-product-detail__spec-row">
                    <dt>{spec.label}</dt>
                    <dd>{spec.value}</dd>
                  </div>
                ))}
              </dl>
            </section>
          )}

          <section className="search-product-detail__section">
            <h3 className="search-product-detail__section-title">商品详情</h3>
            <p className="search-product-detail__description">{description}</p>
          </section>
        </div>

        {externalUrl && (
          <div className="search-product-detail__actions">
            <button
              type="button"
              className="search-product-detail__btn search-product-detail__btn--primary"
              onClick={() => openExternalUrl(externalUrl, item.title, onOpenWebLink)}
            >
              <ShoppingCart className="w-4 h-4" />
              前往购买
            </button>
            <button
              type="button"
              className="search-product-detail__btn search-product-detail__btn--secondary"
              onClick={() => openExternalUrl(externalUrl, item.title, onOpenWebLink)}
            >
              <ExternalLink className="w-4 h-4" />
              查看原页面
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
