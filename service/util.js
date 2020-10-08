// Not a module, because these functions are used in the service worker.
Object.assign(globalThis, {
  deconstructRequestObject,
  deconstructResponseObject,
});

function deconstructRequestObject(request) {
  const {
    url,
    method,
    headers,
    body,
    mode,
    credentials,
    cache,
    redirect,
    referrer,
    integrity,
  } = request;
  const init = {
    method,
    headers: deconstructHeadersObject(headers),
    body,
    mode,
    credentials,
    cache,
    redirect,
    referrer,
    integrity,
  };
  return [url, init];
}

async function deconstructResponseObject(response) {
  const body = await response.arrayBuffer();
  const { status, statusText, headers } = response;
  const init = {
    status,
    statusText,
    headers: deconstructHeadersObject(headers),
  };
  return [body, init];
}

function deconstructHeadersObject(headers) {
  return headers === undefined
    ? undefined
    : Object.fromEntries([...headers.entries()]);
}
