// Moteur 3D du visualiseur de modèles de Revel (D-160) : three.js et les modules utiles,
// exposés en `window.RevelThree` pour `crates/revel-render/src/model.js`.
import * as THREE from "three";
import { GLTFLoader } from "three/examples/jsm/loaders/GLTFLoader.js";
import { DRACOLoader } from "three/examples/jsm/loaders/DRACOLoader.js";
import { KTX2Loader } from "three/examples/jsm/loaders/KTX2Loader.js";
import { HDRLoader } from "three/examples/jsm/loaders/HDRLoader.js";
import { MeshoptDecoder } from "three/examples/jsm/libs/meshopt_decoder.module.js";
import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js";
import { RoomEnvironment } from "three/examples/jsm/environments/RoomEnvironment.js";

window.RevelThree = {
  THREE,
  GLTFLoader,
  DRACOLoader,
  KTX2Loader,
  HDRLoader,
  MeshoptDecoder,
  OrbitControls,
  RoomEnvironment,
};
