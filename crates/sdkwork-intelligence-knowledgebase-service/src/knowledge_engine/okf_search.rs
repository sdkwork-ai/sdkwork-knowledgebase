use sdkwork_knowledgebase_contract::okf::OkfConceptSummary;
use sdkwork_utils_rust::is_blank;
use std::collections::HashMap;

use crate::ports::knowledge_okf_concept_link_store::KnowledgeOkfConceptLinkEdge;

const TITLE_WEIGHT: f64 = 3.0;
const CONCEPT_ID_WEIGHT: f64 = 2.5;
const TAG_WEIGHT: f64 = 1.5;
const DESCRIPTION_WEIGHT: f64 = 1.0;
const BODY_WEIGHT: f64 = 2.0;
const SEGMENT_EXACT_BONUS: f64 = 0.35;
const SEGMENT_PREFIX_BONUS: f64 = 0.15;
const LINK_OUTBOUND_BOOST: f64 = 0.3;
const LINK_INBOUND_BOOST: f64 = 0.2;
const ANCHOR_MATCH_BOOST: f64 = 0.45;
const DEFAULT_SNIPPET_CHARS: usize = 320;

pub fn normalize_query(query: &str) -> Vec<String> {
    query
        .to_lowercase()
        .split(|character: char| !character.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(str::to_string)
        .collect()
}

fn normalize_identifier(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect()
}

fn field_match_score(field: &str, tokens: &[String]) -> f64 {
    if tokens.is_empty() {
        return 0.0;
    }

    let normalized_field = normalize_identifier(field);
    let matches = tokens
        .iter()
        .filter(|token| {
            let normalized_token = normalize_identifier(token);
            !normalized_token.is_empty()
                && (field.to_lowercase().contains(token.as_str())
                    || normalized_field.contains(&normalized_token))
        })
        .count();

    matches as f64 / tokens.len() as f64
}

fn segment_match_bonus(concept_id: &str, tokens: &[String]) -> f64 {
    if tokens.is_empty() {
        return 0.0;
    }

    let segments = concept_id.split('/').collect::<Vec<_>>();
    let mut bonus = 0.0;

    for token in tokens {
        let normalized_token = normalize_identifier(token);
        if normalized_token.is_empty() {
            continue;
        }

        for segment in &segments {
            let segment_lower = segment.to_lowercase();
            let normalized_segment = normalize_identifier(segment);
            if normalized_segment == normalized_token {
                bonus += SEGMENT_EXACT_BONUS;
            } else if normalized_segment.starts_with(&normalized_token)
                || normalized_token.starts_with(&normalized_segment)
            {
                bonus += SEGMENT_PREFIX_BONUS;
            } else if segment_lower.contains(token.as_str()) {
                bonus += SEGMENT_PREFIX_BONUS * 0.5;
            }
        }
    }

    bonus.min(1.0)
}

pub fn rank_okf_concept(page: &OkfConceptSummary, tokens: &[String]) -> f64 {
    if tokens.is_empty() {
        return 0.5;
    }

    let title_score = field_match_score(&page.title, tokens) * TITLE_WEIGHT;
    let concept_id_score = field_match_score(&page.concept_id, tokens) * CONCEPT_ID_WEIGHT;
    let tag_score = field_match_score(&page.tags.join(" "), tokens) * TAG_WEIGHT;
    let description_score = field_match_score(&page.description, tokens) * DESCRIPTION_WEIGHT;
    let weighted = (title_score + concept_id_score + tag_score + description_score)
        / (TITLE_WEIGHT + CONCEPT_ID_WEIGHT + TAG_WEIGHT + DESCRIPTION_WEIGHT);
    let segment_bonus = segment_match_bonus(&page.concept_id, tokens);
    let base_score = weighted + segment_bonus;
    let recency_bonus = if base_score > 0.0 {
        page.source_count as f64 * 0.01
    } else {
        0.0
    };

    (base_score + recency_bonus).clamp(0.0, 1.5)
}

pub fn rank_okf_concepts(
    pages: Vec<OkfConceptSummary>,
    query: &str,
    top_k: u32,
) -> Vec<(f64, OkfConceptSummary)> {
    let normalized_query = normalize_query(query);
    let mut ranked = rank_okf_concepts_with_tokens(pages, &normalized_query);
    ranked.truncate(top_k.max(1) as usize);
    ranked
}

pub fn rank_okf_concepts_with_tokens(
    pages: Vec<OkfConceptSummary>,
    tokens: &[String],
) -> Vec<(f64, OkfConceptSummary)> {
    let mut ranked = pages
        .into_iter()
        .map(|page| (rank_okf_concept(&page, tokens), page))
        .filter(|(score, _)| *score > 0.0 || tokens.is_empty())
        .collect::<Vec<_>>();

    ranked.sort_by(|left, right| {
        right
            .0
            .partial_cmp(&left.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.1.concept_id.cmp(&right.1.concept_id))
    });

    ranked
}

pub fn expand_ranked_with_link_edges(
    mut ranked: Vec<(f64, OkfConceptSummary)>,
    edges: &[KnowledgeOkfConceptLinkEdge],
    summaries_by_id: &HashMap<String, OkfConceptSummary>,
    tokens: &[String],
    max_candidates: usize,
) -> Vec<(f64, OkfConceptSummary)> {
    if edges.is_empty() || ranked.is_empty() || max_candidates == 0 {
        return ranked;
    }

    // Index incident edges by endpoint and lowercase each anchor exactly once.
    // Expansion then costs O(candidates + incident edges) string work instead of
    // O(seeds x edges) with one lowercased allocation per pair.
    let mut outbound: HashMap<&str, Vec<usize>> = HashMap::new();
    let mut inbound: HashMap<&str, Vec<usize>> = HashMap::new();
    for (index, edge) in edges.iter().enumerate() {
        outbound
            .entry(edge.from_concept_id.as_str())
            .or_default()
            .push(index);
        inbound
            .entry(edge.to_concept_id.as_str())
            .or_default()
            .push(index);
    }
    let anchor_text_lower = edges
        .iter()
        .map(|edge| edge.anchor_text.to_lowercase())
        .collect::<Vec<_>>();

    let mut score_by_id = ranked
        .iter()
        .map(|(score, concept)| (concept.concept_id.clone(), *score))
        .collect::<HashMap<_, _>>();

    let mut seeds = ranked
        .iter()
        .filter(|(score, _)| *score > 0.0)
        .map(|(score, concept)| (*score, concept.concept_id.clone()))
        .collect::<Vec<_>>();
    seeds.sort_by(|left, right| left.1.cmp(&right.1));
    seeds.dedup_by(|left, right| left.1 == right.1);

    for (score, concept_id) in seeds {
        if let Some(indices) = outbound.get(concept_id.as_str()) {
            for &index in indices {
                let edge = &edges[index];
                let anchor_bonus = tokens.iter().any(|token| {
                    !token.is_empty() && anchor_text_lower[index].contains(token.as_str())
                });
                let boost = score * LINK_OUTBOUND_BOOST
                    + if anchor_bonus {
                        ANCHOR_MATCH_BOOST
                    } else {
                        0.0
                    };
                apply_link_boost(
                    &mut score_by_id,
                    &edge.to_concept_id,
                    boost,
                    summaries_by_id,
                    &mut ranked,
                    max_candidates,
                );
            }
        }
        if let Some(indices) = inbound.get(concept_id.as_str()) {
            for &index in indices {
                let edge = &edges[index];
                apply_link_boost(
                    &mut score_by_id,
                    &edge.from_concept_id,
                    score * LINK_INBOUND_BOOST,
                    summaries_by_id,
                    &mut ranked,
                    max_candidates,
                );
            }
        }
    }

    ranked.sort_by(|left, right| {
        right
            .0
            .partial_cmp(&left.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.1.concept_id.cmp(&right.1.concept_id))
    });
    ranked.truncate(max_candidates);
    ranked
}

pub fn combine_metadata_and_body_score(metadata_score: f64, body_score: f64) -> f64 {
    if metadata_score <= 0.0 && body_score <= 0.0 {
        return 0.0;
    }
    let weighted = metadata_score + body_score * BODY_WEIGHT;
    (weighted / (1.0 + BODY_WEIGHT)).clamp(0.0, 1.5)
}

pub fn strip_okf_frontmatter(markdown: &str) -> &str {
    let trimmed = markdown.trim_start();
    if !trimmed.starts_with("---") {
        return markdown;
    }
    let rest = trimmed.strip_prefix("---").unwrap_or(trimmed);
    let Some(end) = rest.find("\n---") else {
        return markdown;
    };
    rest[end + 4..].trim_start()
}

pub fn body_match_score(body: &str, tokens: &[String]) -> f64 {
    if tokens.is_empty() {
        return 0.0;
    }
    let body = strip_okf_frontmatter(body).to_lowercase();
    let matches = tokens
        .iter()
        .filter(|token| !token.is_empty() && body.contains(token.as_str()))
        .count();
    matches as f64 / tokens.len() as f64
}

pub fn extract_body_snippet(body: &str, query: &str, max_len: usize) -> String {
    let normalized = strip_okf_frontmatter(body).replace('\r', "");
    let tokens = normalize_query(query);
    if tokens.is_empty() {
        return truncate_chars(&normalized, max_len);
    }

    let lower = normalized.to_lowercase();
    let match_pos = tokens
        .iter()
        .find_map(|token| lower.find(token.as_str()))
        .unwrap_or(0);
    let start = match_pos.saturating_sub(48);
    let mut snippet = normalized
        .chars()
        .skip(start)
        .take(max_len)
        .collect::<String>();
    if start > 0 {
        snippet = format!("…{snippet}");
    }
    if normalized.chars().count() > start + max_len {
        snippet.push('…');
    }
    snippet
}

pub fn snippet_for_concept(description: &str, body: Option<&str>, query: &str) -> String {
    if let Some(body) = body {
        let body_snippet = extract_body_snippet(body, query, DEFAULT_SNIPPET_CHARS);
        if !is_blank(Some(body_snippet.as_str())) {
            return body_snippet;
        }
    }
    description.trim().to_string()
}

fn apply_link_boost(
    score_by_id: &mut HashMap<String, f64>,
    concept_id: &str,
    boost: f64,
    summaries_by_id: &HashMap<String, OkfConceptSummary>,
    ranked: &mut Vec<(f64, OkfConceptSummary)>,
    max_candidates: usize,
) {
    if boost <= 0.0 {
        return;
    }
    if let Some(score) = score_by_id.get_mut(concept_id) {
        *score += boost;
        let updated_score = *score;
        if let Some(entry) = ranked
            .iter_mut()
            .find(|(_, concept)| concept.concept_id == concept_id)
        {
            entry.0 = updated_score;
        }
        return;
    }
    if ranked.len() >= max_candidates {
        return;
    }
    let Some(summary) = summaries_by_id.get(concept_id) else {
        return;
    };
    score_by_id.insert(concept_id.to_string(), boost);
    ranked.push((boost, summary.clone()));
}

fn truncate_chars(value: &str, max_len: usize) -> String {
    let mut snippet = value.chars().take(max_len).collect::<String>();
    if value.chars().count() > max_len {
        snippet.push('…');
    }
    snippet
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdkwork_knowledgebase_contract::okf::OkfConceptSummary;

    fn sample_concept(concept_id: &str, title: &str, description: &str) -> OkfConceptSummary {
        OkfConceptSummary {
            title: title.to_string(),
            concept_id: concept_id.to_string(),
            concept_type: "Knowledge Concept".to_string(),
            logical_path: format!("okf/{concept_id}.md"),
            bundle_relative_path: format!("{concept_id}.md"),
            description: description.to_string(),
            source_count: 1,
            updated_at: "2026-06-01T00:00:00Z".to_string(),
            tags: vec!["demo".to_string()],
        }
    }

    #[test]
    fn rank_okf_concepts_matches_concept_id_tokens() {
        let pages = vec![
            sample_concept("tables/users", "Users Table", "User dimension table"),
            sample_concept("tables/votes", "Votes Table", "Vote fact table"),
        ];

        let ranked = rank_okf_concepts(pages, "users", 5);

        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].1.concept_id, "tables/users");
    }

    #[test]
    fn rank_okf_concepts_prefers_title_match_over_description_only() {
        let pages = vec![
            sample_concept("tables/users", "Users Table", "User dimension table"),
            sample_concept(
                "tables/posts",
                "Posts Table",
                "Contains users activity history",
            ),
        ];

        let ranked = rank_okf_concepts(pages, "users", 5);

        assert_eq!(ranked.len(), 2);
        assert_eq!(ranked[0].1.concept_id, "tables/users");
        assert!(ranked[0].0 > ranked[1].0);
    }

    #[test]
    fn rank_okf_concepts_treats_hyphen_and_underscore_as_equivalent() {
        let pages = vec![sample_concept(
            "posts/tag_wiki",
            "Tag Wiki",
            "Tag metadata concept",
        )];

        let ranked = rank_okf_concepts(pages, "tag-wiki", 5);

        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].1.concept_id, "posts/tag_wiki");
    }

    #[test]
    fn extract_body_snippet_prefers_query_context() {
        let body = "---\ntype: Entity\ntitle: Users\n---\n# Users\n\nOwnership rules apply here.";
        let snippet = extract_body_snippet(body, "ownership", 40);
        assert!(snippet.to_lowercase().contains("ownership"));
    }

    #[test]
    fn expand_ranked_with_link_edges_boosts_linked_targets() {
        use crate::ports::knowledge_okf_concept_link_store::KnowledgeOkfConceptLinkEdge;

        let pages = [
            sample_concept("tables/users", "Users", "User dimension table"),
            sample_concept("tables/orders", "Orders", "Order fact table"),
        ];
        let summaries = pages
            .iter()
            .map(|page| (page.concept_id.clone(), page.clone()))
            .collect();
        let ranked = vec![(0.8, pages[0].clone())];
        let edges = vec![KnowledgeOkfConceptLinkEdge {
            from_concept_id: "tables/users".to_string(),
            to_concept_id: "tables/orders".to_string(),
            anchor_text: "orders".to_string(),
        }];

        let expanded = expand_ranked_with_link_edges(ranked, &edges, &summaries, &[], 4);

        assert!(expanded
            .iter()
            .any(|(_, page)| page.concept_id == "tables/orders"));
    }
}
