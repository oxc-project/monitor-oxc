import test from "node:test"
import assert from "node:assert"

import { camelCase } from "@luca/cases"
test("@luca/cases", () => {
  assert.equal(camelCase("hello world"), "helloWorld")
})

import { Hono } from '@hono/hono'
test("@hono/hono", async () => {
  const app = new Hono()
  app.get('/posts', (c) => c.text('Many posts'))
  const res = await app.request('/posts')
  assert.equal(res.status, 200)
  assert.equal(await res.text(), 'Many posts')
})
