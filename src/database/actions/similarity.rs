// Currently disabled, we can revisit this in #40
use crate::schema::Product;
use std::collections::HashSet;

pub fn _strip_product_name<'a>(name: String) -> Vec<String> {
    let filter = [
        "Beer",
        "24-pack",
        "PET",
        "tölkki",
        "6-pack",
        "4-pack",
        "8-pack",
        "TIN",
        "PURK",
        "4 x 6-pack",
        "4 x 6 x 33cl",
        "9 x 2 cl",
        "24x50cl",
        "Export",
        "(4x)",
        "III",
    ];

    let _name = filter.iter().fold(name, |mut a, f| {
        a = a.replace(f, "");
        a
    });

    _name
        .split(" ")
        .filter(|s| !(s.ends_with("%") || s.ends_with("cl") || s.ends_with("ml") || *s == "A" || *s == "1" || (s.len() < 5 && s.contains(","))))
        .filter(|s| !s.is_empty())
        .map(|s| s.to_owned())
        .collect()
}

pub fn determine_similarity(product: &Product, other: &Product, tresh: f64) -> (bool, f64) {
    const EMPTY: &String = &String::new();

    let _sim = product.abv == other.abv;
    if !_sim {
        return (false, 0.);
    };

    let _s1 = _strip_product_name(product.name.to_owned());
    let _s2 = _strip_product_name(other.name.to_owned());
    
    if _s1 == _s2 {
        return (_sim, 1.);
    }

    let _score = combined_similarity(&_s1, &_s2, 2, 0.7);

    (_sim && _score >= tresh, _score)
}


/* */

fn word_ngrams(words: &Vec<String>, n: usize) -> HashSet<String> {
    let mut result = HashSet::new();

    if words.len() < n {

        return words.into_iter().map(|w| w.to_owned()).collect();
    }

    for i in 0..=words.len() - n {
        let ngram = words[i..i + n].join(" ");
        result.insert(ngram);
    }

    result
}

fn jaccard_similarity(set1: HashSet<String>, set2: HashSet<String>) -> f64 {
    let intersection = set1.intersection(&set2).count();
    let union = set1.union(&set2).count();

    if union == 0 { 0.0 } else { intersection as f64 / union as f64 }
}

pub fn combined_similarity(s1: &Vec<String>, s2: &Vec<String>, n: usize, alpha: f64) -> f64 {
    let ngrams1 = word_ngrams(s1, n);
    let ngrams2 = word_ngrams(s2, n);

    let jaccard = jaccard_similarity(ngrams1, ngrams2);
    let jw_score = strsim::jaro_winkler(&s1.join(""), &s2.join(""));

    alpha * jaccard + (1.0 - alpha) * jw_score
}
