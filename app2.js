// BUG: because ES modules are only loaded once into an execution environment,
// and the JSON loader plugins also gets registered in 'app1.js'. Thus the
// import statement below does not trigger a fetch, therefore the service
// worker isn't notified, so the JSON loader hook is not detected.

// By using three instead of two question marks, the URLs is now different
// from what is used in 'app1.js', which makes the service worker "see" this
// import statement again.

// This is clearly terrible and needs a better solution.

import "./??? loader: ./loaders/json.js";

import data from "./somedata2.json";
console.log("JSON import 2: ", data);
