//! Historique annuler / rétablir.

use revel_model::Site;
use serde::{Deserialize, Serialize};

use crate::EditError;
use crate::command::{Command, apply, expand};

/// Étape de l'historique : commande appliquée et commande qui l'annule.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Step {
    pub forward: Command,
    pub inverse: Command,
}

/// Historique d'un document ouvert. `limit` : nombre maximal d'étapes conservées.
#[derive(Debug, Clone)]
pub struct History {
    done: Vec<Step>,
    undone: Vec<Step>,
    limit: usize,
    /// La prochaine commande peut être fusionnée avec la dernière (saisie continue).
    open_field: bool,
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
        let cmd = expand(site, &cmd)?;
        let inverse = apply(site, &cmd)?;
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
            self.done.push(Step {
                forward: cmd,
                inverse,
            });
            if self.done.len() > self.limit {
                self.done.remove(0);
            }
        }
        self.open_field = coalesce;
        Ok(applied)
    }

    /// Termine la saisie en cours : la prochaine commande ouvrira une nouvelle étape.
    pub fn seal(&mut self) {
        self.open_field = false;
    }

    /// Annule la dernière étape ; renvoie la commande appliquée pour l'annuler.
    pub fn undo(&mut self, site: &mut Site) -> Result<Option<Command>, EditError> {
        self.open_field = false;
        let Some(step) = self.done.pop() else {
            return Ok(None);
        };
        match apply(site, &step.inverse) {
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
        self.open_field = false;
        let Some(step) = self.undone.pop() else {
            return Ok(None);
        };
        match apply(site, &step.forward) {
            Ok(inverse) => {
                let applied = step.forward.clone();
                self.done.push(Step {
                    forward: step.forward,
                    inverse,
                });
                Ok(Some(applied))
            }
            Err(e) => {
                self.undone.push(step);
                Err(e)
            }
        }
    }
}
