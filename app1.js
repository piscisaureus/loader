// The "./??" prefix is not part of the design; it's rather just a way to make
// it work with ServiceWorker capabilities.

import "./?? loader: ./loaders/json.js";
import "./?? loader: ./loaders/html.js";

import data from "./somedata1.json";
console.log("JSON import 1: ", data);

import frag from "./somesnippet.html";
console.log("HTML import: ", frag);

import * as OrdinaryModule from "./ordinary-module.js";
console.log("Ordinary module import: ", OrdinaryModule);
