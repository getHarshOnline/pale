const protectedLayoutFields = new Set([
  'widgetName', 'type', 'children', 'dynamicBindingPathList',
  'dynamicPropertyPathList', 'dynamicTriggerPathList', 'srcDoc',
  'uncompiledSrcDoc', '__proto__',
]);

function indexWidgets(root) {
  const widgets = new Map();
  function visit(widget, parent) {
    const { widgetName, type, children = [] } = widget;
    if (typeof widgetName !== 'string' || !widgetName || typeof type !== 'string' || !type) {
      throw new Error('Native layout requires named, typed widgets.');
    }
    if (widgets.has(widgetName)) {
      throw new Error(`Duplicate widget name: ${widgetName}`);
    }
    widgets.set(widgetName, { widget, parent });
    for (const child of children ?? []) visit(child, widgetName);
  }
  visit(root, null);
  return widgets;
}

function carryField(widget, saved, key) {
  if (typeof key !== 'string' || protectedLayoutFields.has(key)) {
    throw new Error(`Field cannot be carried as layout: ${String(key)}`);
  }
  if (Object.hasOwn(saved, key)) {
    widget[key] = structuredClone(saved[key]);
  } else {
    delete widget[key];
  }
}

// The caller selects the native revision and the geometry and presentation fields.
export function preserveNativeLayout(generated, native, { geometry, presentation }) {
  const current = indexWidgets(native);
  const next = indexWidgets(generated);
  if (current.size !== next.size) {
    throw new Error('Native widget set changed; reconcile the layout before generation.');
  }
  for (const [name, { widget, parent }] of next) {
    const saved = current.get(name);
    if (!saved) throw new Error('Native widget set changed; reconcile the layout before generation.');
    if (saved.widget.type !== widget.type) throw new Error(`Native widget type changed: ${name}`);
    if (saved.parent !== parent) throw new Error(`Native widget parent changed: ${name}`);
  }

  // Prepare every widget before changing the caller's tree.
  const replacements = [];
  for (const [name, { widget: original }] of next) {
    const widget = { ...original };
    replacements.push({ original, widget });
    const saved = current.get(name).widget;
    for (const key of geometry) carryField(widget, saved, key);
    for (const key of presentation) {
      carryField(widget, saved, key);
      widget.dynamicBindingPathList = (widget.dynamicBindingPathList ?? [])
        .filter(path => path.key !== key);
      if (typeof widget[key] === 'string' && widget[key].includes('{{')) {
        widget.dynamicBindingPathList.push({ key });
      }
    }
    if (widget.type !== 'CUSTOM_WIDGET') continue;
    for (const field of ['srcDoc', 'uncompiledSrcDoc']) {
      if (!widget[field] || !saved[field]) continue;
      widget[field] = structuredClone(widget[field]);
      for (const key of ['html', 'css']) {
        if (Object.hasOwn(saved[field], key)) {
          widget[field][key] = structuredClone(saved[field][key]);
        }
      }
    }
  }

  for (const { original, widget } of replacements) {
    for (const key of Object.keys(original)) {
      if (!Object.hasOwn(widget, key)) delete original[key];
    }
    for (const [key, value] of Object.entries(widget)) {
      if (!Object.hasOwn(original, key) || !Object.is(original[key], value)) {
        original[key] = value;
      }
    }
  }
  return generated;
}
