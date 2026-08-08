import { Star } from 'lucide-react';

export interface StarRatingProps {
  rating: number;
  max?: number;
  size?: 'sm' | 'md';
}

export function StarRating({ rating, max = 5, size = 'md' }: StarRatingProps) {
  const iconClass = size === 'sm' ? 'w-3 h-3' : 'w-3.5 h-3.5';

  return (
    <span className="search-star-rating" aria-label={`评分 ${rating.toFixed(1)}`}>
      {Array.from({ length: max }, (_, index) => {
        const filled = rating >= index + 0.75;
        return (
          <Star
            key={index}
            className={`${iconClass} ${filled ? 'search-star-rating__star--active' : 'search-star-rating__star'}`}
          />
        );
      })}
    </span>
  );
}
