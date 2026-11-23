import assert from 'node:assert/strict';
import test from 'node:test';
import { preserveNativeLayout } from './appsmith-layout.mjs';

const policy = { geometry: ['topRow'], presentation: ['textColor', 'fontSize'] };

function tree() {
  return {
    widgetName: 'Root',
    type: 'CANVAS',
    children: [
      { widgetName: 'Panel', type: 'CONTAINER', children: [] },
      {
        widgetName: 'Card',
        type: 'CUSTOM_WIDGET',
        topRow: 2,
        textColor: 'black',
        fontSize: '{{size}}',
        dynamicBindingPathList: [{ key: 'fontSize' }, { key: 'defaultModel' }],
        srcDoc: { html: '<div />', css: '.card {}', js: 'renderNew()' },
        defaultModel: '{{newData}}',
      },
    ],
  };
}

test('carries selected layout and presentation while preserving generated behavior', () => {
  const generated = tree();
  const native = tree();
  const card = generated.children[1];
  const saved = native.children[1];
  saved.topRow = 20;
  saved.textColor = '{{palette.color}}';
  delete saved.fontSize;
  saved.srcDoc = { html: '<article />', css: '.card { color: blue }', js: 'renderOld()' };
  saved.defaultModel = '{{oldData}}';

  assert.equal(preserveNativeLayout(generated, native, policy), generated);
  assert.equal(card.topRow, 20);
  assert.equal(card.textColor, '{{palette.color}}');
  assert.equal(Object.hasOwn(card, 'fontSize'), false);
  assert.deepEqual(card.dynamicBindingPathList, [{ key: 'defaultModel' }, { key: 'textColor' }]);
  assert.equal(card.defaultModel, '{{newData}}');
  assert.deepEqual(card.srcDoc, { ...saved.srcDoc, js: 'renderNew()' });
});

test('refuses ambiguous identity and reparenting before any layout change', () => {
  for (const [plant, diagnostic] of [
    [native => { native.children[1].widgetName = 'Panel'; }, /Duplicate widget name/],
    [native => { native.children[0].children.push(native.children.pop()); }, /parent changed/],
    [native => { native.children[1].type = 'TEXT'; }, /type changed/],
  ]) {
    const generated = tree();
    const native = tree();
    native.topRow = 50;
    plant(native);
    assert.throws(() => preserveNativeLayout(generated, native, policy), diagnostic);
    assert.deepEqual(generated, tree());
  }
});

test('a late binding or custom widget failure preserves the generated tree', () => {
  for (const plant of [
    widget => { widget.dynamicBindingPathList = [null]; },
    widget => { widget.srcDoc = 'invalid custom widget document'; },
  ]) {
    const generated = tree();
    const native = tree();
    native.topRow = 50;
    native.children[1].topRow = 20;
    plant(generated.children[1]);
    const original = structuredClone(generated);

    assert.throws(() => preserveNativeLayout(generated, native, policy), TypeError);
    assert.deepEqual(generated, original);
  }
});

test('refuses to carry widget structure, binding metadata or whole custom documents', () => {
  for (const field of [
    'widgetName', 'type', 'children', 'dynamicBindingPathList',
    'dynamicPropertyPathList', 'dynamicTriggerPathList', 'srcDoc',
    'uncompiledSrcDoc', '__proto__',
  ]) {
    for (const selection of ['geometry', 'presentation']) {
      const generated = tree();
      const native = tree();
      native.children[1].defaultModel = '{{oldData}}';
      native.children[1].srcDoc.js = 'renderOld()';
      const options = { ...policy, [selection]: [...policy[selection], field] };
      assert.throws(
        () => preserveNativeLayout(generated, native, options),
        /Field cannot be carried as layout/,
      );
      assert.deepEqual(generated, tree());
    }
  }
});
