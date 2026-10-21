//! Historique annuler / rétablir.

use revel_model::Site;
use serde::{Deserialize, Serialize};

use crate::EditError;
use crate::command::{Command, apply, expand};

/// Exécutant d'une commande : l'applique et renvoie son inverse.
pub type Exec<'a> = dyn FnMut(&mut Site, &Command) -> Result<Command, EditError> + 'a;

/// Commandes d'une suite, ou la commande seule.
fn flatten(c: Command) -> Vec<Command> {
    match c {
        Command::Batch { commands } => commands,
        other => vec![other],
    }
}

/// Étape de l'historique : commande appliquée et commande qui l'annule.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Step {
    pub forward: Command,
    pub inverse: Command,
    /// Un fichier qu'elle écrit (contenu, données) a été changé par une version reçue (D-108).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub stale: bool,
}

impl Step {
    fn new(forward: Command, inverse: Command) -> Self {
        Self {
            forward,
            inverse,
            stale: false,
        }
    }
}

/// Commandes d'une suite, suites imbriquées comprises.
fn all(c: &Command) -> Vec<&Command> {
    match c {
        Command::Batch { commands } => commands.iter().flat_map(all).collect(),
        other => vec![other],
    }
}

/// La commande écrit un des fichiers `files` (chemins relatifs au site) : contenus du CMS,
/// schéma, données, fichiers déplacés, textes liés.
fn writes(c: &Command, files: &[String]) -> bool {
    let entry = |folder: &str, file: &str| {
        let prefix = format!("{}/{file}.", folder.trim_end_matches('/'));
        files.iter().any(|f| f.starts_with(&prefix))
    };
    let data = || {
        files.iter().any(|f| {
            f != "site.revel.json"
                && [".json", ".yml", ".yaml", ".md", ".toml"]
                    .iter()
                    .any(|e| f.ends_with(e))
        })
    };
    all(c).into_iter().any(|c| match c {
        Command::SetEntryField { folder, file, .. }
        | Command::SetEntryBody { folder, file, .. }
        | Command::CreateEntry { folder, file, .. }
        | Command::WriteEntry { folder, file, .. }
        | Command::ReplaceInEntry { folder, file, .. } => entry(folder, file),
        Command::SetCmsField { .. }
        | Command::AddCmsField { .. }
        | Command::RemoveCmsField { .. }
        | Command::MoveCmsField { .. }
        | Command::SetCmsCollection { .. }
        | Command::CreateCmsCollection { .. }
        | Command::RemoveCmsCollection { .. }
        | Command::WriteCmsConfig { .. } => files.iter().any(|f| f.ends_with(".pages.yml")),
        Command::MoveFiles { moves } => moves
            .iter()
            .any(|(a, b)| files.contains(a) || files.contains(b)),
        Command::SetDataValue { .. } | Command::WriteDataFile { .. } | Command::SetBound { .. } => {
            data()
        }
        _ => false,
    })
}

/// Historique d'un document ouvert. `limit` : nombre maximal d'étapes conservées.
#[derive(Debug, Clone)]
pub struct History {
    done: Vec<Step>,
    undone: Vec<Step>,
    limit: usize,
    /// La prochaine commande peut être fusionnée avec la dernière (saisie continue).
    open_field: bool,
    /// Groupe de la dernière étape (voir [`History::apply_grouped`]).
    group: Option<String>,
    /// Des versions reçues d'ailleurs ont changé le document (D-108) : chaque étape est vérifiée
    /// avant d'être annulée ou rétablie.
    foreign: bool,
}

impl Default for History {
    fn default() -> Self {
        Self::new(500)
    }
}

impl History {
    pub fn new(limit: usize) -> Self {
        Self {
            done: Vec::new(),
            undone: Vec::new(),
            limit: limit.max(1),
            open_field: false,
            group: None,
            foreign: false,
        }
    }

    /// Versions reçues d'un collègue (D-108) : l'historique est gardé ; les étapes qui écrivent un
    /// fichier changé par elles (`files`) ne seront plus appliquées, les autres seront vérifiées.
    pub fn received(&mut self, files: &[String]) {
        self.foreign = true;
        self.open_field = false;
        self.group = None;
        for st in self.done.iter_mut().chain(self.undone.iter_mut()) {
            if writes(&st.forward, files) || writes(&st.inverse, files) {
                st.stale = true;
            }
        }
    }

    /// L'étape peut encore être appliquée (`back` : l'annuler) sans toucher à ce qu'un collègue a
    /// changé : appliquée puis défaite sur une copie, le document doit revenir à l'identique.
    /// Une commande qui écrit des fichiers n'est jugée que par `stale`.
    fn still_applies(&self, site: &Site, st: &Step, back: bool) -> bool {
        if !self.foreign {
            return true;
        }
        if st.stale {
            return false;
        }
        let (go, ret) = if back {
            (&st.inverse, &st.forward)
        } else {
            (&st.forward, &st.inverse)
        };
        let mut a = site.clone();
        match apply(&mut a, go) {
            Ok(_) => {}
            Err(EditError::External | EditError::Bound(_)) => return true,
            Err(_) => return false,
        }
        let mut b = a;
        match apply(&mut b, ret) {
            Ok(_) => b == *site,
            Err(EditError::External | EditError::Bound(_)) => true,
            Err(_) => false,
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.done.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.undone.is_empty()
    }

    /// Applique une commande et l'enregistre. `coalesce` : fusionner avec la commande précédente
    /// si elle porte sur le même champ (une seule étape pour une saisie continue).
    /// Renvoie la commande concrète appliquée (intentions développées, voir [`expand`]).
    pub fn apply(
        &mut self,
        site: &mut Site,
        cmd: Command,
        coalesce: bool,
    ) -> Result<Command, EditError> {
        self.apply_with(site, cmd, coalesce, &mut |s, c| apply(s, c))
    }

    /// Comme [`History::apply`], avec l'exécutant des commandes fourni par l'appelant (le cœur y
    /// traite les commandes qui écrivent des fichiers, comme `SetBound`).
    pub fn apply_with(
        &mut self,
        site: &mut Site,
        cmd: Command,
        coalesce: bool,
        exec: &mut Exec<'_>,
    ) -> Result<Command, EditError> {
        let cmd = expand(site, &cmd)?;
        let inverse = exec(site, &cmd)?;
        let applied = cmd.clone();
        self.undone.clear();
        if coalesce
            && self.open_field
            && let Some(last) = self.done.last_mut()
            && last.forward.same_field(&cmd)
        {
            // L'inverse d'origine (état d'avant la saisie) est conservé.
            last.forward = cmd;
        } else {
            self.done.push(Step::new(cmd, inverse));
            if self.done.len() > self.limit {
                self.done.remove(0);
            }
        }
        self.open_field = coalesce;
        self.group = None;
        Ok(applied)
    }

    /// Comme [`History::apply_with`], les commandes d'un même `group` appliquées à la suite
    /// formant une seule étape (traduction automatique faite par lots : une seule annulation).
    pub fn apply_grouped(
        &mut self,
        site: &mut Site,
        cmd: Command,
        group: &str,
        exec: &mut Exec<'_>,
    ) -> Result<Command, EditError> {
        let cmd = expand(site, &cmd)?;
        let inverse = exec(site, &cmd)?;
        let applied = cmd.clone();
        self.undone.clear();
        self.open_field = false;
        if self.group.as_deref() == Some(group)
            && let Some(last) = self.done.last_mut()
        {
            let forward = std::mem::replace(&mut last.forward, Command::Batch { commands: vec![] });
            let before = std::mem::replace(&mut last.inverse, Command::Batch { commands: vec![] });
            let mut fw = flatten(forward);
            fw.push(cmd);
            let mut inv = vec![inverse];
            inv.extend(flatten(before));
            last.forward = Command::Batch { commands: fw };
            last.inverse = Command::Batch { commands: inv };
        } else {
            self.done.push(Step::new(cmd, inverse));
            if self.done.len() > self.limit {
                self.done.remove(0);
            }
            self.group = Some(group.to_string());
        }
        Ok(applied)
    }

    /// Termine la saisie en cours : la prochaine commande ouvrira une nouvelle étape.
    pub fn seal(&mut self) {
        self.open_field = false;
        self.group = None;
    }

    /// Annule la dernière étape ; renvoie la commande appliquée pour l'annuler.
    pub fn undo(&mut self, site: &mut Site) -> Result<Option<Command>, EditError> {
        self.undo_with(site, &mut |s, c| apply(s, c))
    }

    pub fn undo_with(
        &mut self,
        site: &mut Site,
        exec: &mut Exec<'_>,
    ) -> Result<Option<Command>, EditError> {
        self.open_field = false;
        self.group = None;
        let Some(step) = self.done.pop() else {
            return Ok(None);
        };
        // Endroit changé depuis par un collègue : l'étape est abandonnée (la suivante reste).
        if !self.still_applies(site, &step, true) {
            return Err(EditError::Outdated);
        }
        match exec(site, &step.inverse) {
            Ok(_) => {
                let applied = step.inverse.clone();
                self.undone.push(step);
                Ok(Some(applied))
            }
            Err(e) => {
                self.done.push(step);
                Err(e)
            }
        }
    }

    /// Rétablit la dernière étape annulée ; renvoie la commande appliquée.
    pub fn redo(&mut self, site: &mut Site) -> Result<Option<Command>, EditError> {
        self.redo_with(site, &mut |s, c| apply(s, c))
    }

    pub fn redo_with(
        &mut self,
        site: &mut Site,
        exec: &mut Exec<'_>,
    ) -> Result<Option<Command>, EditError> {
        self.open_field = false;
        self.group = None;
        let Some(step) = self.undone.pop() else {
            return Ok(None);
        };
        if !self.still_applies(site, &step, false) {
            return Err(EditError::Outdated);
        }
        match exec(site, &step.forward) {
            Ok(inverse) => {
                let applied = step.forward.clone();
                self.done.push(Step::new(step.forward, inverse));
                Ok(Some(applied))
            }
            Err(e) => {
                self.undone.push(step);
                Err(e)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::address::{Owner, Parent, Target};
    use revel_model::{Attr, Node, Page};

    fn el(id: &str, class: &str) -> Node {
        Node::Element {
            id: Some(id.into()),
            tag: "p".into(),
            attrs: vec![Attr {
                name: "class".into(),
                value: Some(class.into()),
                when: None,
            }],
            children: vec![],
        }
    }

    fn site() -> Site {
        let mut s = Site::new("Essai", "fr");
        s.pages = vec![Page {
            id: "index".into(),
            path: "/".into(),
            layout: None,
            props: Default::default(),
            computed: Default::default(),
            body: vec![el("a", "a"), el("b", "b"), el("c", "c")],
            draft: false,
            name: None,
            code: Default::default(),
        }];
        s
    }

    fn at(id: &str) -> Target {
        Target {
            owner: Owner::Page("index".into()),
            node: id.into(),
        }
    }

    fn class(id: &str, v: &str) -> Command {
        Command::SetAttr {
            at: at(id),
            name: "class".into(),
            value: Some(v.into()),
        }
    }

    fn body() -> Parent {
        Parent {
            owner: Owner::Page("index".into()),
            node: None,
            branch: Default::default(),
        }
    }

    #[test]
    fn annuler_apres_une_version_recue() {
        let mut s = site();
        let mut h = History::default();
        h.apply(&mut s, class("a", "mienne"), false).unwrap();
        h.apply(&mut s, class("b", "mienne"), false).unwrap();
        // Version reçue : le collègue change `c` et `b`.
        apply(&mut s, &class("c", "collegue")).unwrap();
        apply(&mut s, &class("b", "collegue")).unwrap();
        h.received(&["site.revel.json".to_string()]);
        // `b` : changé depuis par le collègue, l'étape est abandonnée.
        assert!(matches!(h.undo(&mut s), Err(EditError::Outdated)));
        // `a` : toujours à moi, annulé ; le changement du collègue reste.
        assert!(h.undo(&mut s).unwrap().is_some());
        let classes: Vec<String> = s.pages[0]
            .body
            .iter()
            .map(|n| match n {
                Node::Element { attrs, .. } => attrs[0].value.clone().unwrap(),
                _ => String::new(),
            })
            .collect();
        assert_eq!(classes, vec!["a", "collegue", "collegue"]);
        assert!(!h.can_undo());
    }

    #[test]
    fn position_decalee_par_un_collegue() {
        let mut s = site();
        let mut h = History::default();
        h.apply(
            &mut s,
            Command::Insert {
                parent: body(),
                index: 2,
                node: el("n", "nouveau"),
            },
            false,
        )
        .unwrap();
        // Le collègue ajoute un élément au début : l'élément ajouté est maintenant en 3e place.
        apply(
            &mut s,
            &Command::Insert {
                parent: body(),
                index: 0,
                node: el("x", "collegue"),
            },
        )
        .unwrap();
        h.received(&["site.revel.json".to_string()]);
        let before = s.clone();
        assert!(matches!(h.undo(&mut s), Err(EditError::Outdated)));
        assert_eq!(s, before);
    }

    #[test]
    fn contenu_ecrit_par_un_collegue() {
        let mut h = History::default();
        h.done.push(Step::new(
            Command::SetEntryField {
                folder: "src/content/projets".into(),
                file: "force".into(),
                key: "title".into(),
                value: Some("A".into()),
            },
            Command::SetEntryField {
                folder: "src/content/projets".into(),
                file: "force".into(),
                key: "title".into(),
                value: Some("B".into()),
            },
        ));
        h.received(&["src/content/projets/autre.md".to_string()]);
        assert!(!h.done[0].stale);
        h.received(&["src/content/projets/force.md".to_string()]);
        assert!(h.done[0].stale);
        let mut s = site();
        assert!(matches!(h.undo(&mut s), Err(EditError::Outdated)));
    }
}
