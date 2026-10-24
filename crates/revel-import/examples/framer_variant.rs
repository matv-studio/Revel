//! Rendu d'une variante d'un composant Framer lu dans un module : `framer_variant <module.mjs> <nom> <variante>`.
use revel_import::framer_jsx::{Out, Render, parse_at};
use std::collections::HashMap;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let js = std::fs::read_to_string(&a[1]).unwrap();
    let end = js.find(&format!("displayName=`{}`", a[2])).unwrap();
    let st = js[..end].rfind("cycleOrder:").unwrap();
    let start = js[..st]
        .rfind("variantClassNames")
        .map(|x| x.saturating_sub(4000))
        .unwrap_or(0);
    let seg = &js[start..end];
    let ids: Vec<String> = regex::Regex::new(r"\{Default:`(\w+)`,Expanded:`(\w+)`\}")
        .unwrap()
        .captures(seg)
        .map(|c| vec![c[1].to_string(), c[2].to_string()])
        .unwrap();
    let state = regex::Regex::new(&format!(r"\(\)=>([\w$]+)(?:===|!==)`{}`", ids[1]))
        .unwrap()
        .captures(seg)
        .unwrap()[1]
        .to_string();
    let mut r = Render {
        variant: if a[3] == "e" { &ids[1] } else { &ids[0] },
        conds: HashMap::new(),
        variants: ids.clone(),
        tags: HashMap::new(),
        texts: HashMap::new(),
    };
    r.read_defs(seg, &state);
    let co = seg.rfind("cycleOrder:").unwrap();
    let ret = seg[co..].find("return ").unwrap() + co + 7;
    let (e, _) = parse_at(seg, ret);
    let mut out = Out::default();
    let nodes = r.render(&e, &mut out, false);
    println!(
        "state={state} conds={:?} tags={:?}\ntaps={:?}",
        r.conds, r.tags, out.taps
    );
    println!("{}", serde_json::to_string_pretty(&nodes).unwrap().len());
    fn pr(n: &revel_model::Node, d: usize) {
        match n {
            revel_model::Node::Element {
                tag,
                attrs,
                children,
                ..
            } => {
                println!(
                    "{}<{} {}>",
                    "  ".repeat(d),
                    tag,
                    attrs
                        .iter()
                        .map(|a| format!(
                            "{}={:?}",
                            a.name,
                            a.value
                                .as_deref()
                                .unwrap_or("")
                                .chars()
                                .take(60)
                                .collect::<String>()
                        ))
                        .collect::<Vec<_>>()
                        .join(" ")
                );
                for c in children {
                    pr(c, d + 1)
                }
            }
            revel_model::Node::Text { text } => println!("{}{text:?}", "  ".repeat(d)),
            _ => {}
        }
    }
    for n in &nodes {
        pr(n, 0)
    }
}
