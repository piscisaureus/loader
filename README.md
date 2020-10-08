# Module loader hook prototype

### How to run it.

- Probably works with Chrome only.
- Have developer tools open. The "console", "sources", and "network" tabs are
  going to be the interesting ones.

#### Local

- Use any static http server to statically serve the top level directory.
- Navigate to http://localhost:«port»/index.html.

#### Online

- Go to https://piscisaureus.github.io/loader/.

### What are these files?

- `app1.js`, `app2.js` - The application's "main" modules; they're imported by
  `index.html`.
- `somedata.json`, `somesnippet.html` - Non JavaScript files that can be
  imported as modules.
- `loaders/*` - The JSON and HTML loader plugins live here. Take a look!
- `service/*` - The infrastructure that makes loader plugins work. It's a mess,
  don't look.
