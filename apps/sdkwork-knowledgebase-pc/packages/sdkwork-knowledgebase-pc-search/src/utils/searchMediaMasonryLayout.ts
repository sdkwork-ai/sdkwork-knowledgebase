export const MASONRY_GAP_PX = 12;

export const MASONRY_MIN_COLUMN_WIDTH = {
  image: 148,
  video: 172,
  product: 164
} as const;

export type MasonryMediaVariant = keyof typeof MASONRY_MIN_COLUMN_WIDTH;

export interface MasonryPlacement {
  column: number;
  top: number;
  height: number;
}

export interface MasonryLayout {
  columnCount: number;
  columnWidth: number;
  containerHeight: number;
  placements: MasonryPlacement[];
}

export function resolveMasonryColumnCount(
  containerWidth: number,
  minColumnWidth: number,
  gap = MASONRY_GAP_PX
): number {
  if (containerWidth <= 0) return 1;
  return Math.max(1, Math.floor((containerWidth + gap) / (minColumnWidth + gap)));
}

export function resolveMasonryColumnWidth(
  containerWidth: number,
  columnCount: number,
  gap = MASONRY_GAP_PX
): number {
  if (columnCount <= 0) return containerWidth;
  return (containerWidth - gap * (columnCount - 1)) / columnCount;
}

/** Estimate thumbnail height from media aspect ratio within a masonry column. */
export function estimateMasonryThumbHeight(
  columnWidth: number,
  mediaWidth: number,
  mediaHeight: number
): number {
  if (columnWidth <= 0) return 0;
  if (mediaWidth <= 0 || mediaHeight <= 0) {
    return columnWidth * 0.625;
  }

  const ratio = mediaHeight / mediaWidth;
  const natural = columnWidth * ratio;
  const minHeight = columnWidth * 0.42;
  const maxHeight = columnWidth * 1.95;
  return Math.round(Math.max(minHeight, Math.min(maxHeight, natural)));
}

export function computeMasonryPlacements(
  itemHeights: number[],
  columnCount: number,
  gap = MASONRY_GAP_PX
): MasonryPlacement[] {
  const columnTops = Array.from({ length: columnCount }, () => 0);

  return itemHeights.map((height) => {
    let column = 0;
    for (let index = 1; index < columnCount; index += 1) {
      if (columnTops[index] < columnTops[column]) {
        column = index;
      }
    }

    const top = columnTops[column];
    columnTops[column] += height + gap;
    return { column, top, height };
  });
}

export function computeMasonryContainerHeight(
  placements: MasonryPlacement[],_gap = MASONRY_GAP_PX
): number {
  if (placements.length === 0) return 0;

  const columnCount = Math.max(...placements.map((placement) => placement.column)) + 1;
  const columnBottoms = Array.from({ length: columnCount }, () => 0);

  placements.forEach((placement) => {
    columnBottoms[placement.column] = Math.max(
      columnBottoms[placement.column],
      placement.top + placement.height
    );
  });

  return Math.max(...columnBottoms);
}

export function buildMasonryLayout(
  containerWidth: number,
  itemHeights: number[],
  minColumnWidth: number,
  gap = MASONRY_GAP_PX
): MasonryLayout {
  const columnCount = resolveMasonryColumnCount(containerWidth, minColumnWidth, gap);
  const columnWidth = resolveMasonryColumnWidth(containerWidth, columnCount, gap);
  const placements = computeMasonryPlacements(itemHeights, columnCount, gap);
  const containerHeight = computeMasonryContainerHeight(placements, gap);

  return {
    columnCount,
    columnWidth,
    containerHeight,
    placements
  };
}
