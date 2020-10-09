export default async function (request) {
  let response = await fetch(request);
  if (
    !response.redirected &&
    response.headers.get("Content-Type").startsWith("application/json")
  ) {
    // Convert JSON file to module.
    return new Response(`export default ${await response.text()}`, {
      headers: { "Content-Type": "application/javascript" },
    });
  } else {
    // Pass through non-JSON responses unmodified.
    return response;
  }
}
