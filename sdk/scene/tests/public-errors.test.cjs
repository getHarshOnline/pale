"use strict";

const assert = require("node:assert/strict");
const { test } = require("node:test");
const { ContactError, GeometryError, SceneBudgetError } = require("@rtepl/pale-scene");
const {
  OrganizationIntakeError,
  OrganizationSceneError,
  OrganizationTopologyError,
} = require("@rtepl/pale-scene/organization");
const { WorkspaceChangeIntakeError } = require("@rtepl/pale-scene/change");

test("public Scene errors keep their exported class names", () => {
  const errors = [
    new GeometryError("input"),
    new ContactError("input"),
    new SceneBudgetError("limits"),
    new OrganizationIntakeError("closed"),
    new OrganizationTopologyError("bounds"),
    new OrganizationSceneError("closed"),
    new WorkspaceChangeIntakeError({ kind: "foreign-occurrence" }),
  ];

  for (const error of errors) {
    assert.equal(error.name, error.constructor.name);
    assert(error instanceof Error);
  }
});
