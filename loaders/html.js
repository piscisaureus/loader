export default async function (request) {
  let response = await fetch(request);
  if (response.headers.get("Content-Type").startsWith("text/html")) {
    // Convert HTML snippet to a module that exports a DOM DocumentFragment.
    // WARNING: demo material only. This will execute scripts!
    const source = `\
      const html = ${JSON.stringify(await response.text())};
      const fragment = document
        .createRange()
        .createContextualFragment(html);
      export default fragment;
    `;
    return new Response(source, {
      headers: { "Content-Type": "application/javascript" },
    });
  } else {
    // Pass through non-HTML responses unmodified.
    return response;
  }
}
