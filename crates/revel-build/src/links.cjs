// Revel : chargé par Node avant le projet (NODE_OPTIONS --require) pendant une construction.
// Windows refuse de créer un lien symbolique sans droits d'administrateur ni « Mode
// développeur » (EPERM) ; certains disques ne savent pas en créer. Le lien refusé est remplacé
// par une jonction (dossier, aucun droit requis) ou par une copie (fichier). Exemple :
// @sveltejs/adapter-vercel relie les fonctions `.func` entre elles.
"use strict";
const fs = require("fs");
const path = require("path");

if (!fs.__revelLinks) {
  fs.__revelLinks = true;
  const origSync = fs.symlinkSync;
  const refused = (e) => !!e && ["EPERM", "EACCES", "ENOSYS", "ENOTSUP"].includes(e.code);
  // Cible absolue : relative au dossier du lien, comme pour un lien symbolique.
  const absolute = (target, link) =>
    path.resolve(path.dirname(path.resolve(String(link))), String(target));
  const copyDir = (from, to) => fs.cpSync(from, to, { recursive: true, dereference: true });

  function fallback(target, link, type, err) {
    if (type === "junction") throw err;
    const abs = absolute(target, link);
    let st = null;
    try {
      st = fs.statSync(abs);
    } catch (_) {
      st = null;
    }
    if ((st && st.isDirectory()) || (!st && type === "dir")) {
      try {
        origSync(abs, link, "junction");
        return;
      } catch (e) {
        if (!st) throw err;
        copyDir(abs, link);
        return;
      }
    }
    if (st) {
      fs.copyFileSync(abs, link);
      return;
    }
    throw err;
  }

  fs.symlinkSync = function symlinkSync(target, link, type) {
    try {
      return origSync.call(fs, target, link, type);
    } catch (e) {
      if (!refused(e)) throw e;
      return fallback(target, link, type, e);
    }
  };

  const origCb = fs.symlink;
  fs.symlink = function symlink(target, link, type, cb) {
    if (typeof type === "function") {
      cb = type;
      type = undefined;
    }
    origCb.call(fs, target, link, type, (e) => {
      if (!refused(e)) return cb(e);
      try {
        fallback(target, link, type, e);
        cb(null);
      } catch (e2) {
        cb(e2);
      }
    });
  };

  const promises = fs.promises;
  const origP = promises.symlink;
  promises.symlink = async function symlink(target, link, type) {
    try {
      return await origP.call(promises, target, link, type);
    } catch (e) {
      if (!refused(e)) throw e;
      return fallback(target, link, type, e);
    }
  };

  // Imports nommés (`import { symlinkSync } from "node:fs"`) à jour.
  try {
    require("module").syncBuiltinESMExports();
  } catch (_) {
    /* Node ancien */
  }
}
