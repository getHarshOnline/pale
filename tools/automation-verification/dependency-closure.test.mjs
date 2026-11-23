import assert from 'node:assert/strict';
import test from 'node:test';
import { dependencyClosure } from './dependency-closure.mjs';

test('preserves both consumer orders across shared dependencies and cycles', () => {
  const graph = {
    first: ['leaf', 'shared'],
    second: ['shared'],
    leaf: ['first'],
    shared: [],
  };
  const roots = new Set(['first', 'second']);
  for (const [depthFirst, expected] of [
    [false, ['first', 'second', 'leaf', 'shared']],
    [true, ['first', 'leaf', 'shared', 'second']],
  ]) {
    const seen = [];
    const result = dependencyClosure(roots, name => {
      seen.push(name);
      return graph[name];
    }, { depthFirst });
    assert.deepEqual([...result], expected);
    assert.deepEqual(seen, expected);
    assert.deepEqual([...roots], ['first', 'second']);
  }
});

test('a later binding can introduce dependencies into the prior selection', () => {
  const graph = { entry: ['read'], read: [], decode: [] };
  const first = dependencyClosure(['entry'], name => graph[name]);
  assert.deepEqual([...first], ['entry', 'read']);
  graph.read = ['decode'];
  assert.deepEqual([...dependencyClosure(first, name => graph[name])], ['entry', 'read', 'decode']);
});
