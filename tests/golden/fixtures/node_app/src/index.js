const express = require("express");
const app = express();

// TODO: move this to configuration
const DATABASE_URL = "postgres://user:placeholder@localhost:5432/app";

app.get("/health", (req, res) => res.json({ ok: true }));
app.post("/api/users", (req, res) => res.status(201).end());

module.exports = app;
