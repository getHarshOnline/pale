import assert from 'node:assert/strict';
import test from 'node:test';
import { appsmithFunctionArguments } from './appsmith-arguments.mjs';

test('uses the caller collection and preserves literal and symbolic defaults', () => {
  const action = {
    async open(id, title = 'open, now', options = { size: [1, 2] }, page = this['page']) {},
  };
  assert.deepEqual(appsmithFunctionArguments(action.open, 'Workspace'), [
    { name: 'id' },
    { name: 'title', value: 'open, now' },
    { name: 'options', value: '{{{ size: [1, 2] }}}' },
    { name: 'page', value: "{{Workspace['page']}}" },
  ]);
  assert.deepEqual(appsmithFunctionArguments(
    "function (text='a\\'b', view=this.pick('this.page', obj.this.page)) {}",
    'OtherPage',
  ), [
    { name: 'text', value: "a'b" },
    { name: 'view', value: "{{OtherPage.pick('this.page', obj.this.page)}}" },
  ]);
});

test('leaves expressions symbolic and rejects unsupported parameter syntax', () => {
  const expression = "load(value='text' + (() => { throw Error('executed') })()) {}";
  assert.deepEqual(appsmithFunctionArguments(expression, 'Workspace'), [
    { name: 'value', value: "{{'text' + (() => { throw Error('executed') })()}}" },
  ]);
  for (const source of [
    'input => doWork(input)',
    'load({ value }) {}',
    'load(value=/a,b/) {}',
    'load(value=`${call()}`) {}',
    'load(value=) {}',
  ]) assert.throws(() => appsmithFunctionArguments(source, 'Workspace'));
  assert.throws(() => appsmithFunctionArguments('load() {}', 'Workspace.value'), /collection identifier/);
});
