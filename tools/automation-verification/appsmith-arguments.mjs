const identifier = /^[A-Za-z_$][A-Za-z0-9_$]*/;
const functionHead = /^(?:async\s+)?(?:function\b\s*(?:[A-Za-z_$][A-Za-z0-9_$]*\s*)?|[A-Za-z_$][A-Za-z0-9_$]*\s*)?\(/;
const closing = { '(': ')', '[': ']', '{': '}' };

function isQuote(character) {
  return character === '"' || character === "'" || character === '`';
}

function readLiteral(source, start) {
  const quote = source[start];
  for (let index = start + 1; index < source.length; index += 1) {
    if (source[index] === '\\') {
      index += 1;
    } else if (source[index] === quote) {
      return { text: source.slice(start, index + 1), end: index + 1 };
    } else if (quote === '`' && source.startsWith('${', index)) {
      throw new Error('Interpolated parameter defaults need explicit metadata.');
    }
  }
  throw new Error('Unclosed string in function parameters.');
}

function splitParameters(source) {
  const head = functionHead.exec(source);
  if (!head) throw new Error('Expected a parenthesized function parameter list.');
  let start = head[0].length;
  const parameters = [];
  const stack = [];
  for (let index = start; index < source.length; index += 1) {
    const character = source[index];
    if (isQuote(character)) {
      index = readLiteral(source, index).end - 1;
    } else if (character === '/') {
      throw new Error('Slash syntax in parameter defaults needs explicit metadata.');
    } else if (Object.hasOwn(closing, character)) {
      stack.push(closing[character]);
    } else if (character === ')' && !stack.length) {
      const last = source.slice(start, index).trim();
      if (last) parameters.push(last);
      return parameters;
    } else if (')]}'.includes(character)) {
      if (stack.pop() !== character) throw new Error('Unbalanced function parameters.');
    } else if (character === ',' && !stack.length) {
      parameters.push(source.slice(start, index).trim());
      start = index + 1;
    }
  }
  throw new Error('Unclosed function parameter list.');
}

function bindReceiver(source, collectionName) {
  let result = '';
  let index = 0;
  while (index < source.length) {
    if (isQuote(source[index])) {
      const literal = readLiteral(source, index);
      result += literal.text;
      index = literal.end;
      continue;
    }
    const token = identifier.exec(source.slice(index))?.[0];
    if (!token) {
      result += source[index];
      index += 1;
      continue;
    }
    const previous = source.slice(0, index).trimEnd().at(-1);
    const following = source.slice(index + token.length).trimStart();
    const receiver = token === 'this' && previous !== '.' &&
      (following.startsWith('.') || following.startsWith('['));
    result += receiver ? collectionName : token;
    index += token.length;
  }
  return result;
}

export function appsmithFunctionArguments(fn, collectionName) {
  if (typeof collectionName !== 'string' || identifier.exec(collectionName)?.[0] !== collectionName) {
    throw new Error('Expected a collection identifier.');
  }
  const source = (typeof fn === 'function' ? Function.prototype.toString.call(fn) : fn).trim();
  return splitParameters(source).map(parameter => {
    const match = /^([A-Za-z_$][A-Za-z0-9_$]*)(?:\s*=\s*(\S[\s\S]*))?$/.exec(parameter);
    if (!match) throw new Error(`Unsupported function parameter: ${parameter}`);
    const [, name, value] = match;
    if (value === undefined) return { name };
    if ((value[0] === '"' || value[0] === "'") && readLiteral(value, 0).end === value.length) {
      // Parse only a complete string token. Binding expressions stay as source.
      return { name, value: Function('"use strict"; return (' + value + ');')() };
    }
    return { name, value: '{{' + bindReceiver(value, collectionName) + '}}' };
  });
}
