// If this file gets loaded, the service worker isn't active yet.
navigator.serviceWorker.ready.then(() => {
  window.addEventListener("unload", (event) => {
    console.warn("ServiceWorker installed; reloading page to activate.");
  });
  location.reload();
});
