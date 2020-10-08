// Of course we'd like to use `import` instead, but unfortunately Chrome doesn't
// support modules as service workers...
importScripts("./service//util.js");

self.addEventListener("fetch", (event) => {
  event.respondWith(maybeFetch(event));
});

async function maybeFetch(event) {
  return (await onfetch(event)) ?? (await fetch(event.request));
}

async function onfetch(event) {
  const { clientId, request } = event;

  if (
    request.mode !== "navigate" &&
    !doesReferrerContainFullUrl(
      request.url,
      request.referrer,
      request.referrerPolicy
    )
  ) {
    console.warn(
      `Can't intercept request to ${request.url} because of referrer policy '${request.referrerPolicy}'`
    );
    return;
  }

  const url = new URL(request.url);
  const [search, pathname] = [url.search, url.pathname].map(decodeURIComponent);
  const referrerUrl = new URL(request.referrer || location);

  let loaderImportSpecifier;
  if (pathname.endsWith("/service/verify.js")) {
    // Return an empty module. If the worker isn't active, the actual
    // `verify.js` file is loaded which will reload the page.
    return new Response("export default undefined;", js_headers());
  } else if (/\/service(\.js|\/)/.test(pathname)) {
    // Ignore to stay sane.
  } else if (
    pathname.endsWith("/") &&
    (loaderImportSpecifier = /^[#\?]+\s*loader\s*:\s*(\S.*)$/.exec(search)?.[1])
  ) {
    let loaderUrl = new URL(loaderImportSpecifier, referrerUrl);
    const loaderInfo = getOrCreateLoader(clientId, loaderUrl);
    addLoaderToModule(clientId, referrerUrl, loaderInfo);
    return dummyModule();
  } else if (pathname === "/favicon.ico") {
    // Because all those failing requests annoy me.
    return favicon();
  } else {
    const referrerInfo = getOrCreateModuleInfo(clientId, referrerUrl);
    return referrerInfo.fetch(request);
  }
}

function dummyModule() {
  return new Response(`export default undefined;`, js_headers());
}

function js_headers() {
  return { headers: { "Content-Type": "application/javascript" } };
}

function html_headers() {
  return { headers: { "Content-Type": "text/html" } };
}

let loaderMap = new Map();
let moduleMap = new Map();

function getOrCreateLoader(clientId, loaderUrl) {
  let loaderKey = `${clientId}+${loaderUrl}`;
  let loaderInfo = loaderMap.get(loaderKey);
  if (loaderInfo == null) {
    const load = generateLoaderImpl(loaderUrl);
    loaderInfo = {
      url: loaderUrl,
      load, // Note: this is a promise that resolves to a function.
    };
    loaderMap.set(loaderKey, loaderInfo);
  }
  return loaderInfo;
}

async function generateLoaderImpl(loaderUrl) {
  // === set up loader worker  ===
  const {
    port1: localLoaderFetchPort,
    port2: remoteLoaderFetchPort,
  } = new MessageChannel();

  const {
    port1: localDelegatedFetchPort,
    port2: remoteDelegatedFetchPort,
  } = new MessageChannel();

  const {
    port1: localWorkerReadyNotifyPort,
    port2: remoteWorkerReadyNotifyPort,
  } = new MessageChannel();

  // Forward fetches made by the loader worker to the next `fetch()` handler.
  let delegatedFetch;
  localDelegatedFetchPort.addEventListener("message", async (event) => {
    const {
      data: args,
      ports: [replyPort],
    } = event;
    const response = await delegatedFetch(...args);
    const responseArgs = await deconstructResponseObject(response);
    replyPort.postMessage(responseArgs);
  });
  localDelegatedFetchPort.start();

  // Ask a window to create a Worker for us, as ServiceWorkers can't create workers... :/
  const windowClient = await getWindowClient();
  windowClient.postMessage(
    {
      op: "newWorker",
      url: "./service/sandbox.js",
      options: { type: "module", name: `loader: ${loaderUrl}` },
      message: { loaderUrl: String(loaderUrl) },
    },
    [
      remoteLoaderFetchPort,
      remoteDelegatedFetchPort,
      remoteWorkerReadyNotifyPort,
    ]
  );

  // Wait for the worker to finish initializing.
  await new Promise((res, rej) => {
    localWorkerReadyNotifyPort.addEventListener("message", res, { once: true });
    localWorkerReadyNotifyPort.start();
  });

  // Generate new fetch function that passes requests through the new loader.
  function loaderFetch(request, delegatedFetch_) {
    delegatedFetch = delegatedFetch_;

    const {
      port1: localReplyPort,
      port2: remoteReplyPort,
    } = new MessageChannel();

    const promise = new Promise((resolve, _) => {
      localReplyPort.addEventListener(
        "message",
        (event) => resolve(new Response(...event.data)),
        { once: true }
      );
      localReplyPort.start();
    });

    const message = deconstructRequestObject(request);
    localLoaderFetchPort.postMessage(message, [remoteReplyPort]);

    return promise;
  }

  return loaderFetch;
}

defaultFetchImpl = fetch.bind(globalThis);

async function getWindowClient() {
  let clients = await self.clients.matchAll({ type: "window" });
  let windowClient = clients.shift();
  if (!windowClient) {
    throw new Error("Can't do anything without client Window");
  }
  return windowClient;
}

function getOrCreateModuleInfo(clientId, moduleUrl) {
  let moduleKey = `${clientId}+${moduleUrl}`;
  let moduleInfo = moduleMap.get(moduleKey);
  if (moduleInfo == null) {
    moduleInfo = {
      url: moduleUrl,
      fetch: defaultFetchImpl,
    };
    moduleMap.set(moduleKey, moduleInfo);
  }
  return moduleInfo;
}

function addLoaderToModule(clientId, moduleUrl, loaderInfo) {
  const moduleInfo = getOrCreateModuleInfo(clientId, moduleUrl);
  const { fetch: nextFetch } = moduleInfo;
  const { load } = loaderInfo;
  moduleInfo.fetch = async function fetch(resource, init) {
    let request =
      resource instanceof Request ? resource : new Request(resource, init);
    let load2 = await load;
    return load2(request, nextFetch) ?? nextFetch(request);
  };
}

function doesReferrerContainFullUrl(location, referrer, referrerPolicy) {
  if (!referrer) return false;
  const locationUrl = new URL(location);
  const referrerUrl = new URL(referrer);
  const isSameOrigin = referrerUrl.origin === locationUrl.origin;
  const isDowngrade =
    referrerUrl.protocol === "https:" && locationUrl.protocol === "http:";
  const isFullUrl = {
    "no-referrer": false,
    "no-referrer-when-downgrade": !isDowngrade,
    origin: false,
    "origin-when-cross-origin": isSameOrigin,
    "same-origin": isSameOrigin,
    "strict-origin": false,
    "strict-origin-when-cross-origin": isSameOrigin,
    "unsafe-url": true,
  }[referrerPolicy];
  if (isFullUrl == null) {
    throw new Error(`Unexpected referrerPolicy: '${referrerPolicy}'`);
  }
  return isFullUrl;
}

async function favicon() {
  let data = `\
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
  <text y=".9em" font-size="90">🦕</text>
</svg>`;
  return new Response(data, { headers: { "Content-Type": "image/svg+xml" } });
}
