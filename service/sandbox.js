import "./util.js"; // Not actually a module.

self.addEventListener("message", setup, { once: true });

async function setup(event) {
  const {
    data: { loaderUrl },
    ports: [
      inboundFetchRequestPort,
      outboundFetchRequestPort,
      workerReadyNotifyPort,
    ],
  } = event;

  // Make outbound fetch() work.
  async function outboundFetch(resource, init) {
    let requestArgs;
    if (resource instanceof Request) {
      requestArgs = deconstructRequestObject(resource);
      if (init !== undefined) {
        Object.assign(requestArgs[1], init);
      }
    } else {
      requestArgs = [resource, init];
    }

    const {
      port1: localReplyPort,
      port2: remoteReplyPort,
    } = new MessageChannel();
    outboundFetchRequestPort.postMessage(requestArgs, [remoteReplyPort]);
    return await new Promise((resolve, _) => {
      localReplyPort.addEventListener(
        "message",
        (event) => resolve(new Response(...event.data)),
        { once: true }
      );
      localReplyPort.start();
    });
  }
  self.fetch = outboundFetch;

  // Import the actual loader. Do this after patching the global `fetch()`
  // function, just in case the loader assigns it to a local or something.
  const { default: loaderFetch } = await import(loaderUrl);

  // Handle inbound fetch requests.
  inboundFetchRequestPort.addEventListener("message", async (event) => {
    const {
      data: requestArgs,
      ports: [replyPort],
    } = event;
    const request = new Request(...requestArgs);
    const response = await loaderFetch(request);
    const responseArgs = await deconstructResponseObject(response);
    replyPort.postMessage(responseArgs);
  });

  // Turn on the firehose.
  inboundFetchRequestPort.start();

  // Notify the service worker that this sandbox is ready for action.
  workerReadyNotifyPort.postMessage(true);
}
