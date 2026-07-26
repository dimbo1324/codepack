"""Cross-platform developer scripts, driven by the orchestrator in ``scripts/runner``.

Layout rules, which the orchestrator's config loader partly enforces:

* One directory per script, each with a ``__main__.py`` so ``python -m
  scripts.<name>`` runs it. Small scripts stay a single module; large ones decompose
  freely inside their own directory.
* **Scripts never import each other.** A script is meant to be readable, editable, and
  deletable on its own. The only shared code is ``scripts/_toolkit`` — infrastructure
  (process launching, config loading, output), never anybody's business logic. That
  dependency points one way, the same rule the Rust workspace follows.
* Behaviour lives in each script's own ``config/*.json``, not in its Python. Paths,
  command lines, and thresholds are data.
"""
