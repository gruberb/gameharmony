use std::collections::HashMap;

pub fn calculate_harmony_score(rankings: &HashMap<String, u64>) -> u64 {
    if rankings.is_empty() {
        return 0;
    }

    // Average position score (0-100)
    let position_score: u64 = rankings
        .values()
        .map(|&rank| if rank <= 100 { 101 - rank } else { 0 })
        .sum();

    // Appearance multiplier (1.0 to 2.0)
    // With 5 sites, this gives:
    // 1 site:   no bonus (multiplier 1.0)
    // 2 sites:  25% bonus (multiplier 1.25)
    // 3 sites:  50% bonus (multiplier 1.5)
    // 4 sites:  75% bonus (multiplier 1.75)
    // 5 sites:  100% bonus (multiplier 2.0)
    let appearance_multiplier = 100 + (25 * (rankings.len() - 1)) as u64;

    position_score * appearance_multiplier / 100
}
