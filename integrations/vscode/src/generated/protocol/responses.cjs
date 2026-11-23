/*!
Bundled third-party helper licenses.

--- ajv 8.20.0 ---
The MIT License (MIT)

Copyright (c) 2015-2021 Evgeny Poberezkin

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.



--- ajv-formats 3.0.1 ---
MIT License

Copyright (c) 2020 Evgeny Poberezkin

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

*/
"use strict";
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __esm = (fn, res) => function __init() {
  return fn && (res = (0, fn[__getOwnPropNames(fn)[0]])(fn = 0)), res;
};
var __commonJS = (cb, mod) => function __require() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
var __export = (target, all) => {
  for (var name in all)
    __defProp(target, name, { get: all[name], enumerable: true });
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toCommonJS = (mod) => __copyProps(__defProp({}, "__esModule", { value: true }), mod);

// node_modules/ajv-formats/dist/formats.js
var require_formats = __commonJS({
  "node_modules/ajv-formats/dist/formats.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    exports.formatNames = exports.fastFormats = exports.fullFormats = void 0;
    function fmtDef(validate, compare) {
      return { validate, compare };
    }
    exports.fullFormats = {
      // date: http://tools.ietf.org/html/rfc3339#section-5.6
      date: fmtDef(date, compareDate),
      // date-time: http://tools.ietf.org/html/rfc3339#section-5.6
      time: fmtDef(getTime(true), compareTime),
      "date-time": fmtDef(getDateTime(true), compareDateTime),
      "iso-time": fmtDef(getTime(), compareIsoTime),
      "iso-date-time": fmtDef(getDateTime(), compareIsoDateTime),
      // duration: https://tools.ietf.org/html/rfc3339#appendix-A
      duration: /^P(?!$)((\d+Y)?(\d+M)?(\d+D)?(T(?=\d)(\d+H)?(\d+M)?(\d+S)?)?|(\d+W)?)$/,
      uri,
      "uri-reference": /^(?:[a-z][a-z0-9+\-.]*:)?(?:\/?\/(?:(?:[a-z0-9\-._~!$&'()*+,;=:]|%[0-9a-f]{2})*@)?(?:\[(?:(?:(?:(?:[0-9a-f]{1,4}:){6}|::(?:[0-9a-f]{1,4}:){5}|(?:[0-9a-f]{1,4})?::(?:[0-9a-f]{1,4}:){4}|(?:(?:[0-9a-f]{1,4}:){0,1}[0-9a-f]{1,4})?::(?:[0-9a-f]{1,4}:){3}|(?:(?:[0-9a-f]{1,4}:){0,2}[0-9a-f]{1,4})?::(?:[0-9a-f]{1,4}:){2}|(?:(?:[0-9a-f]{1,4}:){0,3}[0-9a-f]{1,4})?::[0-9a-f]{1,4}:|(?:(?:[0-9a-f]{1,4}:){0,4}[0-9a-f]{1,4})?::)(?:[0-9a-f]{1,4}:[0-9a-f]{1,4}|(?:(?:25[0-5]|2[0-4]\d|[01]?\d\d?)\.){3}(?:25[0-5]|2[0-4]\d|[01]?\d\d?))|(?:(?:[0-9a-f]{1,4}:){0,5}[0-9a-f]{1,4})?::[0-9a-f]{1,4}|(?:(?:[0-9a-f]{1,4}:){0,6}[0-9a-f]{1,4})?::)|[Vv][0-9a-f]+\.[a-z0-9\-._~!$&'()*+,;=:]+)\]|(?:(?:25[0-5]|2[0-4]\d|[01]?\d\d?)\.){3}(?:25[0-5]|2[0-4]\d|[01]?\d\d?)|(?:[a-z0-9\-._~!$&'"()*+,;=]|%[0-9a-f]{2})*)(?::\d*)?(?:\/(?:[a-z0-9\-._~!$&'"()*+,;=:@]|%[0-9a-f]{2})*)*|\/(?:(?:[a-z0-9\-._~!$&'"()*+,;=:@]|%[0-9a-f]{2})+(?:\/(?:[a-z0-9\-._~!$&'"()*+,;=:@]|%[0-9a-f]{2})*)*)?|(?:[a-z0-9\-._~!$&'"()*+,;=:@]|%[0-9a-f]{2})+(?:\/(?:[a-z0-9\-._~!$&'"()*+,;=:@]|%[0-9a-f]{2})*)*)?(?:\?(?:[a-z0-9\-._~!$&'"()*+,;=:@/?]|%[0-9a-f]{2})*)?(?:#(?:[a-z0-9\-._~!$&'"()*+,;=:@/?]|%[0-9a-f]{2})*)?$/i,
      // uri-template: https://tools.ietf.org/html/rfc6570
      "uri-template": /^(?:(?:[^\x00-\x20"'<>%\\^`{|}]|%[0-9a-f]{2})|\{[+#./;?&=,!@|]?(?:[a-z0-9_]|%[0-9a-f]{2})+(?::[1-9][0-9]{0,3}|\*)?(?:,(?:[a-z0-9_]|%[0-9a-f]{2})+(?::[1-9][0-9]{0,3}|\*)?)*\})*$/i,
      // For the source: https://gist.github.com/dperini/729294
      // For test cases: https://mathiasbynens.be/demo/url-regex
      url: /^(?:https?|ftp):\/\/(?:\S+(?::\S*)?@)?(?:(?!(?:10|127)(?:\.\d{1,3}){3})(?!(?:169\.254|192\.168)(?:\.\d{1,3}){2})(?!172\.(?:1[6-9]|2\d|3[0-1])(?:\.\d{1,3}){2})(?:[1-9]\d?|1\d\d|2[01]\d|22[0-3])(?:\.(?:1?\d{1,2}|2[0-4]\d|25[0-5])){2}(?:\.(?:[1-9]\d?|1\d\d|2[0-4]\d|25[0-4]))|(?:(?:[a-z0-9\u{00a1}-\u{ffff}]+-)*[a-z0-9\u{00a1}-\u{ffff}]+)(?:\.(?:[a-z0-9\u{00a1}-\u{ffff}]+-)*[a-z0-9\u{00a1}-\u{ffff}]+)*(?:\.(?:[a-z\u{00a1}-\u{ffff}]{2,})))(?::\d{2,5})?(?:\/[^\s]*)?$/iu,
      email: /^[a-z0-9!#$%&'*+/=?^_`{|}~-]+(?:\.[a-z0-9!#$%&'*+/=?^_`{|}~-]+)*@(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\.)+[a-z0-9](?:[a-z0-9-]*[a-z0-9])?$/i,
      hostname: /^(?=.{1,253}\.?$)[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?(?:\.[a-z0-9](?:[-0-9a-z]{0,61}[0-9a-z])?)*\.?$/i,
      // optimized https://www.safaribooksonline.com/library/view/regular-expressions-cookbook/9780596802837/ch07s16.html
      ipv4: /^(?:(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)\.){3}(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)$/,
      ipv6: /^((([0-9a-f]{1,4}:){7}([0-9a-f]{1,4}|:))|(([0-9a-f]{1,4}:){6}(:[0-9a-f]{1,4}|((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3})|:))|(([0-9a-f]{1,4}:){5}(((:[0-9a-f]{1,4}){1,2})|:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3})|:))|(([0-9a-f]{1,4}:){4}(((:[0-9a-f]{1,4}){1,3})|((:[0-9a-f]{1,4})?:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}))|:))|(([0-9a-f]{1,4}:){3}(((:[0-9a-f]{1,4}){1,4})|((:[0-9a-f]{1,4}){0,2}:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}))|:))|(([0-9a-f]{1,4}:){2}(((:[0-9a-f]{1,4}){1,5})|((:[0-9a-f]{1,4}){0,3}:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}))|:))|(([0-9a-f]{1,4}:){1}(((:[0-9a-f]{1,4}){1,6})|((:[0-9a-f]{1,4}){0,4}:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}))|:))|(:(((:[0-9a-f]{1,4}){1,7})|((:[0-9a-f]{1,4}){0,5}:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}))|:)))$/i,
      regex,
      // uuid: http://tools.ietf.org/html/rfc4122
      uuid: /^(?:urn:uuid:)?[0-9a-f]{8}-(?:[0-9a-f]{4}-){3}[0-9a-f]{12}$/i,
      // JSON-pointer: https://tools.ietf.org/html/rfc6901
      // uri fragment: https://tools.ietf.org/html/rfc3986#appendix-A
      "json-pointer": /^(?:\/(?:[^~/]|~0|~1)*)*$/,
      "json-pointer-uri-fragment": /^#(?:\/(?:[a-z0-9_\-.!$&'()*+,;:=@]|%[0-9a-f]{2}|~0|~1)*)*$/i,
      // relative JSON-pointer: http://tools.ietf.org/html/draft-luff-relative-json-pointer-00
      "relative-json-pointer": /^(?:0|[1-9][0-9]*)(?:#|(?:\/(?:[^~/]|~0|~1)*)*)$/,
      // the following formats are used by the openapi specification: https://spec.openapis.org/oas/v3.0.0#data-types
      // byte: https://github.com/miguelmota/is-base64
      byte,
      // signed 32 bit integer
      int32: { type: "number", validate: validateInt32 },
      // signed 64 bit integer
      int64: { type: "number", validate: validateInt64 },
      // C-type float
      float: { type: "number", validate: validateNumber },
      // C-type double
      double: { type: "number", validate: validateNumber },
      // hint to the UI to hide input strings
      password: true,
      // unchecked string payload
      binary: true
    };
    exports.fastFormats = {
      ...exports.fullFormats,
      date: fmtDef(/^\d\d\d\d-[0-1]\d-[0-3]\d$/, compareDate),
      time: fmtDef(/^(?:[0-2]\d:[0-5]\d:[0-5]\d|23:59:60)(?:\.\d+)?(?:z|[+-]\d\d(?::?\d\d)?)$/i, compareTime),
      "date-time": fmtDef(/^\d\d\d\d-[0-1]\d-[0-3]\dt(?:[0-2]\d:[0-5]\d:[0-5]\d|23:59:60)(?:\.\d+)?(?:z|[+-]\d\d(?::?\d\d)?)$/i, compareDateTime),
      "iso-time": fmtDef(/^(?:[0-2]\d:[0-5]\d:[0-5]\d|23:59:60)(?:\.\d+)?(?:z|[+-]\d\d(?::?\d\d)?)?$/i, compareIsoTime),
      "iso-date-time": fmtDef(/^\d\d\d\d-[0-1]\d-[0-3]\d[t\s](?:[0-2]\d:[0-5]\d:[0-5]\d|23:59:60)(?:\.\d+)?(?:z|[+-]\d\d(?::?\d\d)?)?$/i, compareIsoDateTime),
      // uri: https://github.com/mafintosh/is-my-json-valid/blob/master/formats.js
      uri: /^(?:[a-z][a-z0-9+\-.]*:)(?:\/?\/)?[^\s]*$/i,
      "uri-reference": /^(?:(?:[a-z][a-z0-9+\-.]*:)?\/?\/)?(?:[^\\\s#][^\s#]*)?(?:#[^\\\s]*)?$/i,
      // email (sources from jsen validator):
      // http://stackoverflow.com/questions/201323/using-a-regular-expression-to-validate-an-email-address#answer-8829363
      // http://www.w3.org/TR/html5/forms.html#valid-e-mail-address (search for 'wilful violation')
      email: /^[a-z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?(?:\.[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?)*$/i
    };
    exports.formatNames = Object.keys(exports.fullFormats);
    function isLeapYear(year) {
      return year % 4 === 0 && (year % 100 !== 0 || year % 400 === 0);
    }
    var DATE = /^(\d\d\d\d)-(\d\d)-(\d\d)$/;
    var DAYS = [0, 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    function date(str) {
      const matches = DATE.exec(str);
      if (!matches)
        return false;
      const year = +matches[1];
      const month = +matches[2];
      const day = +matches[3];
      return month >= 1 && month <= 12 && day >= 1 && day <= (month === 2 && isLeapYear(year) ? 29 : DAYS[month]);
    }
    function compareDate(d1, d2) {
      if (!(d1 && d2))
        return void 0;
      if (d1 > d2)
        return 1;
      if (d1 < d2)
        return -1;
      return 0;
    }
    var TIME = /^(\d\d):(\d\d):(\d\d(?:\.\d+)?)(z|([+-])(\d\d)(?::?(\d\d))?)?$/i;
    function getTime(strictTimeZone) {
      return function time(str) {
        const matches = TIME.exec(str);
        if (!matches)
          return false;
        const hr = +matches[1];
        const min = +matches[2];
        const sec = +matches[3];
        const tz = matches[4];
        const tzSign = matches[5] === "-" ? -1 : 1;
        const tzH = +(matches[6] || 0);
        const tzM = +(matches[7] || 0);
        if (tzH > 23 || tzM > 59 || strictTimeZone && !tz)
          return false;
        if (hr <= 23 && min <= 59 && sec < 60)
          return true;
        const utcMin = min - tzM * tzSign;
        const utcHr = hr - tzH * tzSign - (utcMin < 0 ? 1 : 0);
        return (utcHr === 23 || utcHr === -1) && (utcMin === 59 || utcMin === -1) && sec < 61;
      };
    }
    function compareTime(s1, s2) {
      if (!(s1 && s2))
        return void 0;
      const t1 = (/* @__PURE__ */ new Date("2020-01-01T" + s1)).valueOf();
      const t2 = (/* @__PURE__ */ new Date("2020-01-01T" + s2)).valueOf();
      if (!(t1 && t2))
        return void 0;
      return t1 - t2;
    }
    function compareIsoTime(t1, t2) {
      if (!(t1 && t2))
        return void 0;
      const a1 = TIME.exec(t1);
      const a2 = TIME.exec(t2);
      if (!(a1 && a2))
        return void 0;
      t1 = a1[1] + a1[2] + a1[3];
      t2 = a2[1] + a2[2] + a2[3];
      if (t1 > t2)
        return 1;
      if (t1 < t2)
        return -1;
      return 0;
    }
    var DATE_TIME_SEPARATOR = /t|\s/i;
    function getDateTime(strictTimeZone) {
      const time = getTime(strictTimeZone);
      return function date_time(str) {
        const dateTime = str.split(DATE_TIME_SEPARATOR);
        return dateTime.length === 2 && date(dateTime[0]) && time(dateTime[1]);
      };
    }
    function compareDateTime(dt1, dt2) {
      if (!(dt1 && dt2))
        return void 0;
      const d1 = new Date(dt1).valueOf();
      const d2 = new Date(dt2).valueOf();
      if (!(d1 && d2))
        return void 0;
      return d1 - d2;
    }
    function compareIsoDateTime(dt1, dt2) {
      if (!(dt1 && dt2))
        return void 0;
      const [d1, t1] = dt1.split(DATE_TIME_SEPARATOR);
      const [d2, t2] = dt2.split(DATE_TIME_SEPARATOR);
      const res = compareDate(d1, d2);
      if (res === void 0)
        return void 0;
      return res || compareTime(t1, t2);
    }
    var NOT_URI_FRAGMENT = /\/|:/;
    var URI = /^(?:[a-z][a-z0-9+\-.]*:)(?:\/?\/(?:(?:[a-z0-9\-._~!$&'()*+,;=:]|%[0-9a-f]{2})*@)?(?:\[(?:(?:(?:(?:[0-9a-f]{1,4}:){6}|::(?:[0-9a-f]{1,4}:){5}|(?:[0-9a-f]{1,4})?::(?:[0-9a-f]{1,4}:){4}|(?:(?:[0-9a-f]{1,4}:){0,1}[0-9a-f]{1,4})?::(?:[0-9a-f]{1,4}:){3}|(?:(?:[0-9a-f]{1,4}:){0,2}[0-9a-f]{1,4})?::(?:[0-9a-f]{1,4}:){2}|(?:(?:[0-9a-f]{1,4}:){0,3}[0-9a-f]{1,4})?::[0-9a-f]{1,4}:|(?:(?:[0-9a-f]{1,4}:){0,4}[0-9a-f]{1,4})?::)(?:[0-9a-f]{1,4}:[0-9a-f]{1,4}|(?:(?:25[0-5]|2[0-4]\d|[01]?\d\d?)\.){3}(?:25[0-5]|2[0-4]\d|[01]?\d\d?))|(?:(?:[0-9a-f]{1,4}:){0,5}[0-9a-f]{1,4})?::[0-9a-f]{1,4}|(?:(?:[0-9a-f]{1,4}:){0,6}[0-9a-f]{1,4})?::)|[Vv][0-9a-f]+\.[a-z0-9\-._~!$&'()*+,;=:]+)\]|(?:(?:25[0-5]|2[0-4]\d|[01]?\d\d?)\.){3}(?:25[0-5]|2[0-4]\d|[01]?\d\d?)|(?:[a-z0-9\-._~!$&'()*+,;=]|%[0-9a-f]{2})*)(?::\d*)?(?:\/(?:[a-z0-9\-._~!$&'()*+,;=:@]|%[0-9a-f]{2})*)*|\/(?:(?:[a-z0-9\-._~!$&'()*+,;=:@]|%[0-9a-f]{2})+(?:\/(?:[a-z0-9\-._~!$&'()*+,;=:@]|%[0-9a-f]{2})*)*)?|(?:[a-z0-9\-._~!$&'()*+,;=:@]|%[0-9a-f]{2})+(?:\/(?:[a-z0-9\-._~!$&'()*+,;=:@]|%[0-9a-f]{2})*)*)(?:\?(?:[a-z0-9\-._~!$&'()*+,;=:@/?]|%[0-9a-f]{2})*)?(?:#(?:[a-z0-9\-._~!$&'()*+,;=:@/?]|%[0-9a-f]{2})*)?$/i;
    function uri(str) {
      return NOT_URI_FRAGMENT.test(str) && URI.test(str);
    }
    var BYTE = /^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/gm;
    function byte(str) {
      BYTE.lastIndex = 0;
      return BYTE.test(str);
    }
    var MIN_INT32 = -(2 ** 31);
    var MAX_INT32 = 2 ** 31 - 1;
    function validateInt32(value) {
      return Number.isInteger(value) && value <= MAX_INT32 && value >= MIN_INT32;
    }
    function validateInt64(value) {
      return Number.isInteger(value);
    }
    function validateNumber() {
      return true;
    }
    var Z_ANCHOR = /[^\\]\\Z/;
    function regex(str) {
      if (Z_ANCHOR.test(str))
        return false;
      try {
        new RegExp(str);
        return true;
      } catch (e) {
        return false;
      }
    }
  }
});

// number-formats.mjs
var number_formats_exports = {};
__export(number_formats_exports, {
  numberFormats: () => numberFormats
});
var unsigned, numberFormats;
var init_number_formats = __esm({
  "number-formats.mjs"() {
    unsigned = (maximum) => ({
      type: "number",
      validate: (value) => Number.isSafeInteger(value) && value >= 0 && value <= maximum
    });
    numberFormats = Object.freeze({
      uint16: unsigned(65535),
      uint32: unsigned(4294967295),
      // Bound the text before BigInt; never round an editor coordinate through Number.
      "uint64-decimal": {
        type: "string",
        validate: (value) => typeof value === "string" && value.length >= 1 && value.length <= 20 && /^(0|[1-9][0-9]*)$/u.test(value) && BigInt(value) <= 0xffffffffffffffffn
      }
    });
  }
});

// node_modules/ajv/dist/runtime/ucs2length.js
var require_ucs2length = __commonJS({
  "node_modules/ajv/dist/runtime/ucs2length.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    function ucs2length(str) {
      const len = str.length;
      let length = 0;
      let pos = 0;
      let value;
      while (pos < len) {
        length++;
        value = str.charCodeAt(pos++);
        if (value >= 55296 && value <= 56319 && pos < len) {
          value = str.charCodeAt(pos);
          if ((value & 64512) === 56320)
            pos++;
        }
      }
      return length;
    }
    exports.default = ucs2length;
    ucs2length.code = 'require("ajv/dist/runtime/ucs2length").default';
  }
});

// <stdin>
var stdin_exports = {};
__export(stdin_exports, {
  validateAnalysisPublication: () => validateAnalysisPublication,
  validateConfigurationImprovementResponse: () => validateConfigurationImprovementResponse,
  validateEffectiveProfileExplanationResponse: () => validateEffectiveProfileExplanationResponse,
  validateEffectiveProfileResponse: () => validateEffectiveProfileResponse,
  validatePaleWorkspaceCapabilities: () => validatePaleWorkspaceCapabilities,
  validateProfileContextsResponse: () => validateProfileContextsResponse
});
module.exports = __toCommonJS(stdin_exports);
var validateAnalysisPublication = validate247;
function validate54(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate54.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  const _errs0 = errors;
  let valid0 = false;
  let passing0 = null;
  const _errs1 = errors;
  if (typeof data !== "string") {
    const err0 = { instancePath, schemaPath: "#/oneOf/0/type", keyword: "type", params: { type: "string" }, message: "must be string" };
    if (vErrors === null) {
      vErrors = [err0];
    } else {
      vErrors.push(err0);
    }
    errors++;
  }
  if ("admitted" !== data) {
    const err1 = { instancePath, schemaPath: "#/oneOf/0/const", keyword: "const", params: { allowedValue: "admitted" }, message: "must be equal to constant" };
    if (vErrors === null) {
      vErrors = [err1];
    } else {
      vErrors.push(err1);
    }
    errors++;
  }
  var _valid0 = _errs1 === errors;
  if (_valid0) {
    valid0 = true;
    passing0 = 0;
  }
  const _errs3 = errors;
  if (typeof data !== "string") {
    const err2 = { instancePath, schemaPath: "#/oneOf/1/type", keyword: "type", params: { type: "string" }, message: "must be string" };
    if (vErrors === null) {
      vErrors = [err2];
    } else {
      vErrors.push(err2);
    }
    errors++;
  }
  if ("rejected" !== data) {
    const err3 = { instancePath, schemaPath: "#/oneOf/1/const", keyword: "const", params: { allowedValue: "rejected" }, message: "must be equal to constant" };
    if (vErrors === null) {
      vErrors = [err3];
    } else {
      vErrors.push(err3);
    }
    errors++;
  }
  var _valid0 = _errs3 === errors;
  if (_valid0 && valid0) {
    valid0 = false;
    passing0 = [passing0, 1];
  } else {
    if (_valid0) {
      valid0 = true;
      passing0 = 1;
    }
  }
  if (!valid0) {
    const err4 = { instancePath, schemaPath: "#/oneOf", keyword: "oneOf", params: { passingSchemas: passing0 }, message: "must match exactly one schema in oneOf" };
    if (vErrors === null) {
      vErrors = [err4];
    } else {
      vErrors.push(err4);
    }
    errors++;
  } else {
    errors = _errs0;
    if (vErrors !== null) {
      if (_errs0) {
        vErrors.length = _errs0;
      } else {
        vErrors = null;
      }
    }
  }
  validate54.errors = vErrors;
  return errors === 0;
}
validate54.evaluated = { "dynamicProps": false, "dynamicItems": false };
var formats0 = { ...require_formats().fullFormats, ...(init_number_formats(), __toCommonJS(number_formats_exports)).numberFormats }["uint64-decimal"];
var func2 = require_ucs2length().default;
function validate56(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate56.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.generation === void 0) {
      const err0 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "generation" }, message: "must have required property 'generation'" };
      if (vErrors === null) {
        vErrors = [err0];
      } else {
        vErrors.push(err0);
      }
      errors++;
    }
    if (data.documentRevision === void 0) {
      const err1 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "documentRevision" }, message: "must have required property 'documentRevision'" };
      if (vErrors === null) {
        vErrors = [err1];
      } else {
        vErrors.push(err1);
      }
      errors++;
    }
    for (const key0 in data) {
      if (!(key0 === "documentRevision" || key0 === "generation")) {
        const err2 = { instancePath, schemaPath: "#/additionalProperties", keyword: "additionalProperties", params: { additionalProperty: key0 }, message: "must NOT have additional properties" };
        if (vErrors === null) {
          vErrors = [err2];
        } else {
          vErrors.push(err2);
        }
        errors++;
      }
    }
    if (data.documentRevision !== void 0) {
      let data0 = data.documentRevision;
      if (typeof data0 === "string") {
        if (func2(data0) > 20) {
          const err3 = { instancePath: instancePath + "/documentRevision", schemaPath: "#/properties/documentRevision/maxLength", keyword: "maxLength", params: { limit: 20 }, message: "must NOT have more than 20 characters" };
          if (vErrors === null) {
            vErrors = [err3];
          } else {
            vErrors.push(err3);
          }
          errors++;
        }
        if (func2(data0) < 1) {
          const err4 = { instancePath: instancePath + "/documentRevision", schemaPath: "#/properties/documentRevision/minLength", keyword: "minLength", params: { limit: 1 }, message: "must NOT have fewer than 1 characters" };
          if (vErrors === null) {
            vErrors = [err4];
          } else {
            vErrors.push(err4);
          }
          errors++;
        }
        if (!formats0.validate(data0)) {
          const err5 = { instancePath: instancePath + "/documentRevision", schemaPath: "#/properties/documentRevision/format", keyword: "format", params: { format: "uint64-decimal" }, message: 'must match format "uint64-decimal"' };
          if (vErrors === null) {
            vErrors = [err5];
          } else {
            vErrors.push(err5);
          }
          errors++;
        }
      } else {
        const err6 = { instancePath: instancePath + "/documentRevision", schemaPath: "#/properties/documentRevision/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err6];
        } else {
          vErrors.push(err6);
        }
        errors++;
      }
    }
    if (data.generation !== void 0) {
      let data1 = data.generation;
      if (typeof data1 === "string") {
        if (func2(data1) > 20) {
          const err7 = { instancePath: instancePath + "/generation", schemaPath: "#/properties/generation/maxLength", keyword: "maxLength", params: { limit: 20 }, message: "must NOT have more than 20 characters" };
          if (vErrors === null) {
            vErrors = [err7];
          } else {
            vErrors.push(err7);
          }
          errors++;
        }
        if (func2(data1) < 1) {
          const err8 = { instancePath: instancePath + "/generation", schemaPath: "#/properties/generation/minLength", keyword: "minLength", params: { limit: 1 }, message: "must NOT have fewer than 1 characters" };
          if (vErrors === null) {
            vErrors = [err8];
          } else {
            vErrors.push(err8);
          }
          errors++;
        }
        if (!formats0.validate(data1)) {
          const err9 = { instancePath: instancePath + "/generation", schemaPath: "#/properties/generation/format", keyword: "format", params: { format: "uint64-decimal" }, message: 'must match format "uint64-decimal"' };
          if (vErrors === null) {
            vErrors = [err9];
          } else {
            vErrors.push(err9);
          }
          errors++;
        }
      } else {
        const err10 = { instancePath: instancePath + "/generation", schemaPath: "#/properties/generation/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err10];
        } else {
          vErrors.push(err10);
        }
        errors++;
      }
    }
  } else {
    const err11 = { instancePath, schemaPath: "#/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err11];
    } else {
      vErrors.push(err11);
    }
    errors++;
  }
  validate56.errors = vErrors;
  return errors === 0;
}
validate56.evaluated = { "props": true, "dynamicProps": false, "dynamicItems": false };
function validate247(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate247.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.editor === void 0) {
      const err0 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "editor" }, message: "must have required property 'editor'" };
      if (vErrors === null) {
        vErrors = [err0];
      } else {
        vErrors.push(err0);
      }
      errors++;
    }
    if (data.admission === void 0) {
      const err1 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "admission" }, message: "must have required property 'admission'" };
      if (vErrors === null) {
        vErrors = [err1];
      } else {
        vErrors.push(err1);
      }
      errors++;
    }
    for (const key0 in data) {
      if (!(key0 === "admission" || key0 === "editor")) {
        const err2 = { instancePath, schemaPath: "#/additionalProperties", keyword: "additionalProperties", params: { additionalProperty: key0 }, message: "must NOT have additional properties" };
        if (vErrors === null) {
          vErrors = [err2];
        } else {
          vErrors.push(err2);
        }
        errors++;
      }
    }
    if (data.admission !== void 0) {
      if (!validate54(data.admission, { instancePath: instancePath + "/admission", parentData: data, parentDataProperty: "admission", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate54.errors : vErrors.concat(validate54.errors);
        errors = vErrors.length;
      }
    }
    if (data.editor !== void 0) {
      if (!validate56(data.editor, { instancePath: instancePath + "/editor", parentData: data, parentDataProperty: "editor", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate56.errors : vErrors.concat(validate56.errors);
        errors = vErrors.length;
      }
    }
  } else {
    const err3 = { instancePath, schemaPath: "#/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err3];
    } else {
      vErrors.push(err3);
    }
    errors++;
  }
  validate247.errors = vErrors;
  return errors === 0;
}
validate247.evaluated = { "props": true, "dynamicProps": false, "dynamicItems": false };
var validateConfigurationImprovementResponse = validate250;
var schema31 = { "additionalProperties": false, "description": "Borrowed serialized profile coordinate; editor counters are decimal strings.", "properties": { "compositionPlanDigest": { "type": "string" }, "documentRevision": { "format": "uint64-decimal", "maxLength": 20, "minLength": 1, "type": "string" }, "generation": { "format": "uint64-decimal", "maxLength": 20, "minLength": 1, "type": "string" }, "instanceDigest": { "type": "string" }, "packageSemanticIrDigest": { "type": "string" }, "profileDigest": { "type": "string" }, "selectedTopologyDigest": { "type": "string" }, "workspaceAdmissionEvidenceDigest": { "type": "string" }, "workspaceSemanticIrDigest": { "type": "string" } }, "required": ["generation", "documentRevision", "profileDigest", "instanceDigest", "selectedTopologyDigest", "workspaceSemanticIrDigest", "workspaceAdmissionEvidenceDigest", "packageSemanticIrDigest", "compositionPlanDigest"], "type": "object" };
var func1 = Object.prototype.hasOwnProperty;
function validate77(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate77.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.generation === void 0) {
      const err0 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "generation" }, message: "must have required property 'generation'" };
      if (vErrors === null) {
        vErrors = [err0];
      } else {
        vErrors.push(err0);
      }
      errors++;
    }
    if (data.documentRevision === void 0) {
      const err1 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "documentRevision" }, message: "must have required property 'documentRevision'" };
      if (vErrors === null) {
        vErrors = [err1];
      } else {
        vErrors.push(err1);
      }
      errors++;
    }
    if (data.profileDigest === void 0) {
      const err2 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "profileDigest" }, message: "must have required property 'profileDigest'" };
      if (vErrors === null) {
        vErrors = [err2];
      } else {
        vErrors.push(err2);
      }
      errors++;
    }
    if (data.instanceDigest === void 0) {
      const err3 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "instanceDigest" }, message: "must have required property 'instanceDigest'" };
      if (vErrors === null) {
        vErrors = [err3];
      } else {
        vErrors.push(err3);
      }
      errors++;
    }
    if (data.selectedTopologyDigest === void 0) {
      const err4 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "selectedTopologyDigest" }, message: "must have required property 'selectedTopologyDigest'" };
      if (vErrors === null) {
        vErrors = [err4];
      } else {
        vErrors.push(err4);
      }
      errors++;
    }
    if (data.workspaceSemanticIrDigest === void 0) {
      const err5 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "workspaceSemanticIrDigest" }, message: "must have required property 'workspaceSemanticIrDigest'" };
      if (vErrors === null) {
        vErrors = [err5];
      } else {
        vErrors.push(err5);
      }
      errors++;
    }
    if (data.workspaceAdmissionEvidenceDigest === void 0) {
      const err6 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "workspaceAdmissionEvidenceDigest" }, message: "must have required property 'workspaceAdmissionEvidenceDigest'" };
      if (vErrors === null) {
        vErrors = [err6];
      } else {
        vErrors.push(err6);
      }
      errors++;
    }
    if (data.packageSemanticIrDigest === void 0) {
      const err7 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "packageSemanticIrDigest" }, message: "must have required property 'packageSemanticIrDigest'" };
      if (vErrors === null) {
        vErrors = [err7];
      } else {
        vErrors.push(err7);
      }
      errors++;
    }
    if (data.compositionPlanDigest === void 0) {
      const err8 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "compositionPlanDigest" }, message: "must have required property 'compositionPlanDigest'" };
      if (vErrors === null) {
        vErrors = [err8];
      } else {
        vErrors.push(err8);
      }
      errors++;
    }
    for (const key0 in data) {
      if (!func1.call(schema31.properties, key0)) {
        const err9 = { instancePath, schemaPath: "#/additionalProperties", keyword: "additionalProperties", params: { additionalProperty: key0 }, message: "must NOT have additional properties" };
        if (vErrors === null) {
          vErrors = [err9];
        } else {
          vErrors.push(err9);
        }
        errors++;
      }
    }
    if (data.compositionPlanDigest !== void 0) {
      if (typeof data.compositionPlanDigest !== "string") {
        const err10 = { instancePath: instancePath + "/compositionPlanDigest", schemaPath: "#/properties/compositionPlanDigest/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err10];
        } else {
          vErrors.push(err10);
        }
        errors++;
      }
    }
    if (data.documentRevision !== void 0) {
      let data1 = data.documentRevision;
      if (typeof data1 === "string") {
        if (func2(data1) > 20) {
          const err11 = { instancePath: instancePath + "/documentRevision", schemaPath: "#/properties/documentRevision/maxLength", keyword: "maxLength", params: { limit: 20 }, message: "must NOT have more than 20 characters" };
          if (vErrors === null) {
            vErrors = [err11];
          } else {
            vErrors.push(err11);
          }
          errors++;
        }
        if (func2(data1) < 1) {
          const err12 = { instancePath: instancePath + "/documentRevision", schemaPath: "#/properties/documentRevision/minLength", keyword: "minLength", params: { limit: 1 }, message: "must NOT have fewer than 1 characters" };
          if (vErrors === null) {
            vErrors = [err12];
          } else {
            vErrors.push(err12);
          }
          errors++;
        }
        if (!formats0.validate(data1)) {
          const err13 = { instancePath: instancePath + "/documentRevision", schemaPath: "#/properties/documentRevision/format", keyword: "format", params: { format: "uint64-decimal" }, message: 'must match format "uint64-decimal"' };
          if (vErrors === null) {
            vErrors = [err13];
          } else {
            vErrors.push(err13);
          }
          errors++;
        }
      } else {
        const err14 = { instancePath: instancePath + "/documentRevision", schemaPath: "#/properties/documentRevision/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err14];
        } else {
          vErrors.push(err14);
        }
        errors++;
      }
    }
    if (data.generation !== void 0) {
      let data2 = data.generation;
      if (typeof data2 === "string") {
        if (func2(data2) > 20) {
          const err15 = { instancePath: instancePath + "/generation", schemaPath: "#/properties/generation/maxLength", keyword: "maxLength", params: { limit: 20 }, message: "must NOT have more than 20 characters" };
          if (vErrors === null) {
            vErrors = [err15];
          } else {
            vErrors.push(err15);
          }
          errors++;
        }
        if (func2(data2) < 1) {
          const err16 = { instancePath: instancePath + "/generation", schemaPath: "#/properties/generation/minLength", keyword: "minLength", params: { limit: 1 }, message: "must NOT have fewer than 1 characters" };
          if (vErrors === null) {
            vErrors = [err16];
          } else {
            vErrors.push(err16);
          }
          errors++;
        }
        if (!formats0.validate(data2)) {
          const err17 = { instancePath: instancePath + "/generation", schemaPath: "#/properties/generation/format", keyword: "format", params: { format: "uint64-decimal" }, message: 'must match format "uint64-decimal"' };
          if (vErrors === null) {
            vErrors = [err17];
          } else {
            vErrors.push(err17);
          }
          errors++;
        }
      } else {
        const err18 = { instancePath: instancePath + "/generation", schemaPath: "#/properties/generation/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err18];
        } else {
          vErrors.push(err18);
        }
        errors++;
      }
    }
    if (data.instanceDigest !== void 0) {
      if (typeof data.instanceDigest !== "string") {
        const err19 = { instancePath: instancePath + "/instanceDigest", schemaPath: "#/properties/instanceDigest/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err19];
        } else {
          vErrors.push(err19);
        }
        errors++;
      }
    }
    if (data.packageSemanticIrDigest !== void 0) {
      if (typeof data.packageSemanticIrDigest !== "string") {
        const err20 = { instancePath: instancePath + "/packageSemanticIrDigest", schemaPath: "#/properties/packageSemanticIrDigest/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err20];
        } else {
          vErrors.push(err20);
        }
        errors++;
      }
    }
    if (data.profileDigest !== void 0) {
      if (typeof data.profileDigest !== "string") {
        const err21 = { instancePath: instancePath + "/profileDigest", schemaPath: "#/properties/profileDigest/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err21];
        } else {
          vErrors.push(err21);
        }
        errors++;
      }
    }
    if (data.selectedTopologyDigest !== void 0) {
      if (typeof data.selectedTopologyDigest !== "string") {
        const err22 = { instancePath: instancePath + "/selectedTopologyDigest", schemaPath: "#/properties/selectedTopologyDigest/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err22];
        } else {
          vErrors.push(err22);
        }
        errors++;
      }
    }
    if (data.workspaceAdmissionEvidenceDigest !== void 0) {
      if (typeof data.workspaceAdmissionEvidenceDigest !== "string") {
        const err23 = { instancePath: instancePath + "/workspaceAdmissionEvidenceDigest", schemaPath: "#/properties/workspaceAdmissionEvidenceDigest/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err23];
        } else {
          vErrors.push(err23);
        }
        errors++;
      }
    }
    if (data.workspaceSemanticIrDigest !== void 0) {
      if (typeof data.workspaceSemanticIrDigest !== "string") {
        const err24 = { instancePath: instancePath + "/workspaceSemanticIrDigest", schemaPath: "#/properties/workspaceSemanticIrDigest/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err24];
        } else {
          vErrors.push(err24);
        }
        errors++;
      }
    }
  } else {
    const err25 = { instancePath, schemaPath: "#/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err25];
    } else {
      vErrors.push(err25);
    }
    errors++;
  }
  validate77.errors = vErrors;
  return errors === 0;
}
validate77.evaluated = { "props": true, "dynamicProps": false, "dynamicItems": false };
function validate79(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate79.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.projectId === void 0) {
      const err0 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "projectId" }, message: "must have required property 'projectId'" };
      if (vErrors === null) {
        vErrors = [err0];
      } else {
        vErrors.push(err0);
      }
      errors++;
    }
    if (data.executionWorkspaceId === void 0) {
      const err1 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "executionWorkspaceId" }, message: "must have required property 'executionWorkspaceId'" };
      if (vErrors === null) {
        vErrors = [err1];
      } else {
        vErrors.push(err1);
      }
      errors++;
    }
    for (const key0 in data) {
      if (!(key0 === "executionWorkspaceId" || key0 === "projectId")) {
        const err2 = { instancePath, schemaPath: "#/additionalProperties", keyword: "additionalProperties", params: { additionalProperty: key0 }, message: "must NOT have additional properties" };
        if (vErrors === null) {
          vErrors = [err2];
        } else {
          vErrors.push(err2);
        }
        errors++;
      }
    }
    if (data.executionWorkspaceId !== void 0) {
      if (typeof data.executionWorkspaceId !== "string") {
        const err3 = { instancePath: instancePath + "/executionWorkspaceId", schemaPath: "#/properties/executionWorkspaceId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err3];
        } else {
          vErrors.push(err3);
        }
        errors++;
      }
    }
    if (data.projectId !== void 0) {
      if (typeof data.projectId !== "string") {
        const err4 = { instancePath: instancePath + "/projectId", schemaPath: "#/properties/projectId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err4];
        } else {
          vErrors.push(err4);
        }
        errors++;
      }
    }
  } else {
    const err5 = { instancePath, schemaPath: "#/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err5];
    } else {
      vErrors.push(err5);
    }
    errors++;
  }
  validate79.errors = vErrors;
  return errors === 0;
}
validate79.evaluated = { "props": true, "dynamicProps": false, "dynamicItems": false };
function validate82(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate82.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.baseline === void 0) {
      const err0 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "baseline" }, message: "must have required property 'baseline'" };
      if (vErrors === null) {
        vErrors = [err0];
      } else {
        vErrors.push(err0);
      }
      errors++;
    }
    if (data.candidateEditor === void 0) {
      const err1 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "candidateEditor" }, message: "must have required property 'candidateEditor'" };
      if (vErrors === null) {
        vErrors = [err1];
      } else {
        vErrors.push(err1);
      }
      errors++;
    }
    if (data.candidateSelectedTopologyDigest === void 0) {
      const err2 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "candidateSelectedTopologyDigest" }, message: "must have required property 'candidateSelectedTopologyDigest'" };
      if (vErrors === null) {
        vErrors = [err2];
      } else {
        vErrors.push(err2);
      }
      errors++;
    }
    if (data.candidateProfileDigest === void 0) {
      const err3 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "candidateProfileDigest" }, message: "must have required property 'candidateProfileDigest'" };
      if (vErrors === null) {
        vErrors = [err3];
      } else {
        vErrors.push(err3);
      }
      errors++;
    }
    if (data.candidateInstanceDigest === void 0) {
      const err4 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "candidateInstanceDigest" }, message: "must have required property 'candidateInstanceDigest'" };
      if (vErrors === null) {
        vErrors = [err4];
      } else {
        vErrors.push(err4);
      }
      errors++;
    }
    if (data.derivationReceiptDigest === void 0) {
      const err5 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "derivationReceiptDigest" }, message: "must have required property 'derivationReceiptDigest'" };
      if (vErrors === null) {
        vErrors = [err5];
      } else {
        vErrors.push(err5);
      }
      errors++;
    }
    for (const key0 in data) {
      if (!(key0 === "baseline" || key0 === "candidateEditor" || key0 === "candidateInstanceDigest" || key0 === "candidateProfileDigest" || key0 === "candidateSelectedTopologyDigest" || key0 === "derivationReceiptDigest")) {
        const err6 = { instancePath, schemaPath: "#/additionalProperties", keyword: "additionalProperties", params: { additionalProperty: key0 }, message: "must NOT have additional properties" };
        if (vErrors === null) {
          vErrors = [err6];
        } else {
          vErrors.push(err6);
        }
        errors++;
      }
    }
    if (data.baseline !== void 0) {
      if (!validate77(data.baseline, { instancePath: instancePath + "/baseline", parentData: data, parentDataProperty: "baseline", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate77.errors : vErrors.concat(validate77.errors);
        errors = vErrors.length;
      }
    }
    if (data.candidateEditor !== void 0) {
      if (!validate56(data.candidateEditor, { instancePath: instancePath + "/candidateEditor", parentData: data, parentDataProperty: "candidateEditor", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate56.errors : vErrors.concat(validate56.errors);
        errors = vErrors.length;
      }
    }
    if (data.candidateInstanceDigest !== void 0) {
      if (typeof data.candidateInstanceDigest !== "string") {
        const err7 = { instancePath: instancePath + "/candidateInstanceDigest", schemaPath: "#/properties/candidateInstanceDigest/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err7];
        } else {
          vErrors.push(err7);
        }
        errors++;
      }
    }
    if (data.candidateProfileDigest !== void 0) {
      if (typeof data.candidateProfileDigest !== "string") {
        const err8 = { instancePath: instancePath + "/candidateProfileDigest", schemaPath: "#/properties/candidateProfileDigest/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err8];
        } else {
          vErrors.push(err8);
        }
        errors++;
      }
    }
    if (data.candidateSelectedTopologyDigest !== void 0) {
      if (typeof data.candidateSelectedTopologyDigest !== "string") {
        const err9 = { instancePath: instancePath + "/candidateSelectedTopologyDigest", schemaPath: "#/properties/candidateSelectedTopologyDigest/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err9];
        } else {
          vErrors.push(err9);
        }
        errors++;
      }
    }
    if (data.derivationReceiptDigest !== void 0) {
      if (typeof data.derivationReceiptDigest !== "string") {
        const err10 = { instancePath: instancePath + "/derivationReceiptDigest", schemaPath: "#/properties/derivationReceiptDigest/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err10];
        } else {
          vErrors.push(err10);
        }
        errors++;
      }
    }
  } else {
    const err11 = { instancePath, schemaPath: "#/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err11];
    } else {
      vErrors.push(err11);
    }
    errors++;
  }
  validate82.errors = vErrors;
  return errors === 0;
}
validate82.evaluated = { "props": true, "dynamicProps": false, "dynamicItems": false };
var schema34 = { "additionalProperties": false, "description": "Bounded editor transport over the API-owned preview summary.", "properties": { "affectedCount": { "description": "Number of affected semantic subjects.", "format": "uint32", "minimum": 0, "type": "integer" }, "baselineWorkspaceAdmissionEvidenceDigest": { "description": "Baseline workspace admission-evidence identity.", "type": "string" }, "baselineWorkspaceSemanticIrDigest": { "description": "Baseline workspace semantic identity.", "type": "string" }, "candidateInstanceDigest": { "description": "Candidate private-instance identity.", "type": "string" }, "candidateProfileDigest": { "description": "Candidate effective-profile identity.", "type": "string" }, "candidateWorkspaceAdmissionEvidenceDigest": { "description": "Candidate workspace admission-evidence identity.", "type": "string" }, "candidateWorkspaceSemanticIrDigest": { "description": "Candidate workspace semantic identity.", "type": "string" }, "changeRootCount": { "description": "Number of exact changed roots.", "format": "uint32", "minimum": 0, "type": "integer" }, "compositionPlanChange": { "$ref": "#/$defs/ConfigurationImprovementCoordinateChange", "description": "Exact composition-proof transition." }, "derivationReceiptDigest": { "description": "Exact derivation receipt.", "type": "string" }, "executionWorkspaceId": { "description": "Selected execution workspace.", "type": "string" }, "instanceChange": { "$ref": "#/$defs/ConfigurationImprovementCoordinateChange", "description": "Exact private-instance transition." }, "instanceDigest": { "description": "Baseline private-instance identity.", "type": "string" }, "packageSemanticIrChange": { "$ref": "#/$defs/ConfigurationImprovementCoordinateChange", "description": "Exact selected Package semantic transition." }, "profileChange": { "$ref": "#/$defs/ConfigurationImprovementCoordinateChange", "description": "Exact effective-profile transition." }, "profileDigest": { "description": "Baseline effective-profile identity.", "type": "string" }, "projectId": { "description": "Selected project.", "type": "string" }, "selectedTopologyChange": { "$ref": "#/$defs/ConfigurationImprovementCoordinateChange", "description": "Exact desired-topology transition." }, "unaffectedCount": { "description": "Number of unaffected semantic subjects.", "format": "uint32", "minimum": 0, "type": "integer" }, "workspaceId": { "description": "Principal workspace under review.", "type": "string" } }, "required": ["workspaceId", "projectId", "executionWorkspaceId", "baselineWorkspaceSemanticIrDigest", "baselineWorkspaceAdmissionEvidenceDigest", "profileDigest", "instanceDigest", "candidateProfileDigest", "candidateInstanceDigest", "candidateWorkspaceSemanticIrDigest", "candidateWorkspaceAdmissionEvidenceDigest", "derivationReceiptDigest", "changeRootCount", "affectedCount", "unaffectedCount", "selectedTopologyChange", "profileChange", "instanceChange", "packageSemanticIrChange", "compositionPlanChange"], "type": "object" };
var formats4 = { ...require_formats().fullFormats, ...(init_number_formats(), __toCommonJS(number_formats_exports)).numberFormats }.uint32;
function validate87(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate87.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  const _errs0 = errors;
  let valid0 = false;
  let passing0 = null;
  const _errs1 = errors;
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.state === void 0) {
      const err0 = { instancePath, schemaPath: "#/oneOf/0/required", keyword: "required", params: { missingProperty: "state" }, message: "must have required property 'state'" };
      if (vErrors === null) {
        vErrors = [err0];
      } else {
        vErrors.push(err0);
      }
      errors++;
    }
    if (data.digest === void 0) {
      const err1 = { instancePath, schemaPath: "#/oneOf/0/required", keyword: "required", params: { missingProperty: "digest" }, message: "must have required property 'digest'" };
      if (vErrors === null) {
        vErrors = [err1];
      } else {
        vErrors.push(err1);
      }
      errors++;
    }
    for (const key0 in data) {
      if (!(key0 === "digest" || key0 === "state")) {
        const err2 = { instancePath, schemaPath: "#/oneOf/0/additionalProperties", keyword: "additionalProperties", params: { additionalProperty: key0 }, message: "must NOT have additional properties" };
        if (vErrors === null) {
          vErrors = [err2];
        } else {
          vErrors.push(err2);
        }
        errors++;
      }
    }
    if (data.digest !== void 0) {
      if (typeof data.digest !== "string") {
        const err3 = { instancePath: instancePath + "/digest", schemaPath: "#/oneOf/0/properties/digest/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err3];
        } else {
          vErrors.push(err3);
        }
        errors++;
      }
    }
    if (data.state !== void 0) {
      let data1 = data.state;
      if (typeof data1 !== "string") {
        const err4 = { instancePath: instancePath + "/state", schemaPath: "#/oneOf/0/properties/state/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err4];
        } else {
          vErrors.push(err4);
        }
        errors++;
      }
      if ("unchanged" !== data1) {
        const err5 = { instancePath: instancePath + "/state", schemaPath: "#/oneOf/0/properties/state/const", keyword: "const", params: { allowedValue: "unchanged" }, message: "must be equal to constant" };
        if (vErrors === null) {
          vErrors = [err5];
        } else {
          vErrors.push(err5);
        }
        errors++;
      }
    }
  } else {
    const err6 = { instancePath, schemaPath: "#/oneOf/0/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err6];
    } else {
      vErrors.push(err6);
    }
    errors++;
  }
  var _valid0 = _errs1 === errors;
  if (_valid0) {
    valid0 = true;
    passing0 = 0;
    var props0 = true;
  }
  const _errs8 = errors;
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.state === void 0) {
      const err7 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "state" }, message: "must have required property 'state'" };
      if (vErrors === null) {
        vErrors = [err7];
      } else {
        vErrors.push(err7);
      }
      errors++;
    }
    if (data.prior_digest === void 0) {
      const err8 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "prior_digest" }, message: "must have required property 'prior_digest'" };
      if (vErrors === null) {
        vErrors = [err8];
      } else {
        vErrors.push(err8);
      }
      errors++;
    }
    if (data.candidate_digest === void 0) {
      const err9 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "candidate_digest" }, message: "must have required property 'candidate_digest'" };
      if (vErrors === null) {
        vErrors = [err9];
      } else {
        vErrors.push(err9);
      }
      errors++;
    }
    for (const key1 in data) {
      if (!(key1 === "candidate_digest" || key1 === "prior_digest" || key1 === "state")) {
        const err10 = { instancePath, schemaPath: "#/oneOf/1/additionalProperties", keyword: "additionalProperties", params: { additionalProperty: key1 }, message: "must NOT have additional properties" };
        if (vErrors === null) {
          vErrors = [err10];
        } else {
          vErrors.push(err10);
        }
        errors++;
      }
    }
    if (data.candidate_digest !== void 0) {
      if (typeof data.candidate_digest !== "string") {
        const err11 = { instancePath: instancePath + "/candidate_digest", schemaPath: "#/oneOf/1/properties/candidate_digest/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err11];
        } else {
          vErrors.push(err11);
        }
        errors++;
      }
    }
    if (data.prior_digest !== void 0) {
      if (typeof data.prior_digest !== "string") {
        const err12 = { instancePath: instancePath + "/prior_digest", schemaPath: "#/oneOf/1/properties/prior_digest/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err12];
        } else {
          vErrors.push(err12);
        }
        errors++;
      }
    }
    if (data.state !== void 0) {
      let data4 = data.state;
      if (typeof data4 !== "string") {
        const err13 = { instancePath: instancePath + "/state", schemaPath: "#/oneOf/1/properties/state/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err13];
        } else {
          vErrors.push(err13);
        }
        errors++;
      }
      if ("changed" !== data4) {
        const err14 = { instancePath: instancePath + "/state", schemaPath: "#/oneOf/1/properties/state/const", keyword: "const", params: { allowedValue: "changed" }, message: "must be equal to constant" };
        if (vErrors === null) {
          vErrors = [err14];
        } else {
          vErrors.push(err14);
        }
        errors++;
      }
    }
  } else {
    const err15 = { instancePath, schemaPath: "#/oneOf/1/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err15];
    } else {
      vErrors.push(err15);
    }
    errors++;
  }
  var _valid0 = _errs8 === errors;
  if (_valid0 && valid0) {
    valid0 = false;
    passing0 = [passing0, 1];
  } else {
    if (_valid0) {
      valid0 = true;
      passing0 = 1;
      if (props0 !== true) {
        props0 = true;
      }
    }
  }
  if (!valid0) {
    const err16 = { instancePath, schemaPath: "#/oneOf", keyword: "oneOf", params: { passingSchemas: passing0 }, message: "must match exactly one schema in oneOf" };
    if (vErrors === null) {
      vErrors = [err16];
    } else {
      vErrors.push(err16);
    }
    errors++;
  } else {
    errors = _errs0;
    if (vErrors !== null) {
      if (_errs0) {
        vErrors.length = _errs0;
      } else {
        vErrors = null;
      }
    }
  }
  validate87.errors = vErrors;
  evaluated0.props = props0;
  return errors === 0;
}
validate87.evaluated = { "dynamicProps": true, "dynamicItems": false };
function validate86(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate86.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.workspaceId === void 0) {
      const err0 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "workspaceId" }, message: "must have required property 'workspaceId'" };
      if (vErrors === null) {
        vErrors = [err0];
      } else {
        vErrors.push(err0);
      }
      errors++;
    }
    if (data.projectId === void 0) {
      const err1 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "projectId" }, message: "must have required property 'projectId'" };
      if (vErrors === null) {
        vErrors = [err1];
      } else {
        vErrors.push(err1);
      }
      errors++;
    }
    if (data.executionWorkspaceId === void 0) {
      const err2 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "executionWorkspaceId" }, message: "must have required property 'executionWorkspaceId'" };
      if (vErrors === null) {
        vErrors = [err2];
      } else {
        vErrors.push(err2);
      }
      errors++;
    }
    if (data.baselineWorkspaceSemanticIrDigest === void 0) {
      const err3 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "baselineWorkspaceSemanticIrDigest" }, message: "must have required property 'baselineWorkspaceSemanticIrDigest'" };
      if (vErrors === null) {
        vErrors = [err3];
      } else {
        vErrors.push(err3);
      }
      errors++;
    }
    if (data.baselineWorkspaceAdmissionEvidenceDigest === void 0) {
      const err4 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "baselineWorkspaceAdmissionEvidenceDigest" }, message: "must have required property 'baselineWorkspaceAdmissionEvidenceDigest'" };
      if (vErrors === null) {
        vErrors = [err4];
      } else {
        vErrors.push(err4);
      }
      errors++;
    }
    if (data.profileDigest === void 0) {
      const err5 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "profileDigest" }, message: "must have required property 'profileDigest'" };
      if (vErrors === null) {
        vErrors = [err5];
      } else {
        vErrors.push(err5);
      }
      errors++;
    }
    if (data.instanceDigest === void 0) {
      const err6 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "instanceDigest" }, message: "must have required property 'instanceDigest'" };
      if (vErrors === null) {
        vErrors = [err6];
      } else {
        vErrors.push(err6);
      }
      errors++;
    }
    if (data.candidateProfileDigest === void 0) {
      const err7 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "candidateProfileDigest" }, message: "must have required property 'candidateProfileDigest'" };
      if (vErrors === null) {
        vErrors = [err7];
      } else {
        vErrors.push(err7);
      }
      errors++;
    }
    if (data.candidateInstanceDigest === void 0) {
      const err8 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "candidateInstanceDigest" }, message: "must have required property 'candidateInstanceDigest'" };
      if (vErrors === null) {
        vErrors = [err8];
      } else {
        vErrors.push(err8);
      }
      errors++;
    }
    if (data.candidateWorkspaceSemanticIrDigest === void 0) {
      const err9 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "candidateWorkspaceSemanticIrDigest" }, message: "must have required property 'candidateWorkspaceSemanticIrDigest'" };
      if (vErrors === null) {
        vErrors = [err9];
      } else {
        vErrors.push(err9);
      }
      errors++;
    }
    if (data.candidateWorkspaceAdmissionEvidenceDigest === void 0) {
      const err10 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "candidateWorkspaceAdmissionEvidenceDigest" }, message: "must have required property 'candidateWorkspaceAdmissionEvidenceDigest'" };
      if (vErrors === null) {
        vErrors = [err10];
      } else {
        vErrors.push(err10);
      }
      errors++;
    }
    if (data.derivationReceiptDigest === void 0) {
      const err11 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "derivationReceiptDigest" }, message: "must have required property 'derivationReceiptDigest'" };
      if (vErrors === null) {
        vErrors = [err11];
      } else {
        vErrors.push(err11);
      }
      errors++;
    }
    if (data.changeRootCount === void 0) {
      const err12 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "changeRootCount" }, message: "must have required property 'changeRootCount'" };
      if (vErrors === null) {
        vErrors = [err12];
      } else {
        vErrors.push(err12);
      }
      errors++;
    }
    if (data.affectedCount === void 0) {
      const err13 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "affectedCount" }, message: "must have required property 'affectedCount'" };
      if (vErrors === null) {
        vErrors = [err13];
      } else {
        vErrors.push(err13);
      }
      errors++;
    }
    if (data.unaffectedCount === void 0) {
      const err14 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "unaffectedCount" }, message: "must have required property 'unaffectedCount'" };
      if (vErrors === null) {
        vErrors = [err14];
      } else {
        vErrors.push(err14);
      }
      errors++;
    }
    if (data.selectedTopologyChange === void 0) {
      const err15 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "selectedTopologyChange" }, message: "must have required property 'selectedTopologyChange'" };
      if (vErrors === null) {
        vErrors = [err15];
      } else {
        vErrors.push(err15);
      }
      errors++;
    }
    if (data.profileChange === void 0) {
      const err16 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "profileChange" }, message: "must have required property 'profileChange'" };
      if (vErrors === null) {
        vErrors = [err16];
      } else {
        vErrors.push(err16);
      }
      errors++;
    }
    if (data.instanceChange === void 0) {
      const err17 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "instanceChange" }, message: "must have required property 'instanceChange'" };
      if (vErrors === null) {
        vErrors = [err17];
      } else {
        vErrors.push(err17);
      }
      errors++;
    }
    if (data.packageSemanticIrChange === void 0) {
      const err18 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "packageSemanticIrChange" }, message: "must have required property 'packageSemanticIrChange'" };
      if (vErrors === null) {
        vErrors = [err18];
      } else {
        vErrors.push(err18);
      }
      errors++;
    }
    if (data.compositionPlanChange === void 0) {
      const err19 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "compositionPlanChange" }, message: "must have required property 'compositionPlanChange'" };
      if (vErrors === null) {
        vErrors = [err19];
      } else {
        vErrors.push(err19);
      }
      errors++;
    }
    for (const key0 in data) {
      if (!func1.call(schema34.properties, key0)) {
        const err20 = { instancePath, schemaPath: "#/additionalProperties", keyword: "additionalProperties", params: { additionalProperty: key0 }, message: "must NOT have additional properties" };
        if (vErrors === null) {
          vErrors = [err20];
        } else {
          vErrors.push(err20);
        }
        errors++;
      }
    }
    if (data.affectedCount !== void 0) {
      let data0 = data.affectedCount;
      if (!(typeof data0 == "number" && (!(data0 % 1) && !isNaN(data0)))) {
        const err21 = { instancePath: instancePath + "/affectedCount", schemaPath: "#/properties/affectedCount/type", keyword: "type", params: { type: "integer" }, message: "must be integer" };
        if (vErrors === null) {
          vErrors = [err21];
        } else {
          vErrors.push(err21);
        }
        errors++;
      }
      if (typeof data0 == "number") {
        if (data0 < 0 || isNaN(data0)) {
          const err22 = { instancePath: instancePath + "/affectedCount", schemaPath: "#/properties/affectedCount/minimum", keyword: "minimum", params: { comparison: ">=", limit: 0 }, message: "must be >= 0" };
          if (vErrors === null) {
            vErrors = [err22];
          } else {
            vErrors.push(err22);
          }
          errors++;
        }
        if (!formats4.validate(data0)) {
          const err23 = { instancePath: instancePath + "/affectedCount", schemaPath: "#/properties/affectedCount/format", keyword: "format", params: { format: "uint32" }, message: 'must match format "uint32"' };
          if (vErrors === null) {
            vErrors = [err23];
          } else {
            vErrors.push(err23);
          }
          errors++;
        }
      }
    }
    if (data.baselineWorkspaceAdmissionEvidenceDigest !== void 0) {
      if (typeof data.baselineWorkspaceAdmissionEvidenceDigest !== "string") {
        const err24 = { instancePath: instancePath + "/baselineWorkspaceAdmissionEvidenceDigest", schemaPath: "#/properties/baselineWorkspaceAdmissionEvidenceDigest/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err24];
        } else {
          vErrors.push(err24);
        }
        errors++;
      }
    }
    if (data.baselineWorkspaceSemanticIrDigest !== void 0) {
      if (typeof data.baselineWorkspaceSemanticIrDigest !== "string") {
        const err25 = { instancePath: instancePath + "/baselineWorkspaceSemanticIrDigest", schemaPath: "#/properties/baselineWorkspaceSemanticIrDigest/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err25];
        } else {
          vErrors.push(err25);
        }
        errors++;
      }
    }
    if (data.candidateInstanceDigest !== void 0) {
      if (typeof data.candidateInstanceDigest !== "string") {
        const err26 = { instancePath: instancePath + "/candidateInstanceDigest", schemaPath: "#/properties/candidateInstanceDigest/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err26];
        } else {
          vErrors.push(err26);
        }
        errors++;
      }
    }
    if (data.candidateProfileDigest !== void 0) {
      if (typeof data.candidateProfileDigest !== "string") {
        const err27 = { instancePath: instancePath + "/candidateProfileDigest", schemaPath: "#/properties/candidateProfileDigest/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err27];
        } else {
          vErrors.push(err27);
        }
        errors++;
      }
    }
    if (data.candidateWorkspaceAdmissionEvidenceDigest !== void 0) {
      if (typeof data.candidateWorkspaceAdmissionEvidenceDigest !== "string") {
        const err28 = { instancePath: instancePath + "/candidateWorkspaceAdmissionEvidenceDigest", schemaPath: "#/properties/candidateWorkspaceAdmissionEvidenceDigest/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err28];
        } else {
          vErrors.push(err28);
        }
        errors++;
      }
    }
    if (data.candidateWorkspaceSemanticIrDigest !== void 0) {
      if (typeof data.candidateWorkspaceSemanticIrDigest !== "string") {
        const err29 = { instancePath: instancePath + "/candidateWorkspaceSemanticIrDigest", schemaPath: "#/properties/candidateWorkspaceSemanticIrDigest/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err29];
        } else {
          vErrors.push(err29);
        }
        errors++;
      }
    }
    if (data.changeRootCount !== void 0) {
      let data7 = data.changeRootCount;
      if (!(typeof data7 == "number" && (!(data7 % 1) && !isNaN(data7)))) {
        const err30 = { instancePath: instancePath + "/changeRootCount", schemaPath: "#/properties/changeRootCount/type", keyword: "type", params: { type: "integer" }, message: "must be integer" };
        if (vErrors === null) {
          vErrors = [err30];
        } else {
          vErrors.push(err30);
        }
        errors++;
      }
      if (typeof data7 == "number") {
        if (data7 < 0 || isNaN(data7)) {
          const err31 = { instancePath: instancePath + "/changeRootCount", schemaPath: "#/properties/changeRootCount/minimum", keyword: "minimum", params: { comparison: ">=", limit: 0 }, message: "must be >= 0" };
          if (vErrors === null) {
            vErrors = [err31];
          } else {
            vErrors.push(err31);
          }
          errors++;
        }
        if (!formats4.validate(data7)) {
          const err32 = { instancePath: instancePath + "/changeRootCount", schemaPath: "#/properties/changeRootCount/format", keyword: "format", params: { format: "uint32" }, message: 'must match format "uint32"' };
          if (vErrors === null) {
            vErrors = [err32];
          } else {
            vErrors.push(err32);
          }
          errors++;
        }
      }
    }
    if (data.compositionPlanChange !== void 0) {
      if (!validate87(data.compositionPlanChange, { instancePath: instancePath + "/compositionPlanChange", parentData: data, parentDataProperty: "compositionPlanChange", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate87.errors : vErrors.concat(validate87.errors);
        errors = vErrors.length;
      }
    }
    if (data.derivationReceiptDigest !== void 0) {
      if (typeof data.derivationReceiptDigest !== "string") {
        const err33 = { instancePath: instancePath + "/derivationReceiptDigest", schemaPath: "#/properties/derivationReceiptDigest/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err33];
        } else {
          vErrors.push(err33);
        }
        errors++;
      }
    }
    if (data.executionWorkspaceId !== void 0) {
      if (typeof data.executionWorkspaceId !== "string") {
        const err34 = { instancePath: instancePath + "/executionWorkspaceId", schemaPath: "#/properties/executionWorkspaceId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err34];
        } else {
          vErrors.push(err34);
        }
        errors++;
      }
    }
    if (data.instanceChange !== void 0) {
      if (!validate87(data.instanceChange, { instancePath: instancePath + "/instanceChange", parentData: data, parentDataProperty: "instanceChange", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate87.errors : vErrors.concat(validate87.errors);
        errors = vErrors.length;
      }
    }
    if (data.instanceDigest !== void 0) {
      if (typeof data.instanceDigest !== "string") {
        const err35 = { instancePath: instancePath + "/instanceDigest", schemaPath: "#/properties/instanceDigest/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err35];
        } else {
          vErrors.push(err35);
        }
        errors++;
      }
    }
    if (data.packageSemanticIrChange !== void 0) {
      if (!validate87(data.packageSemanticIrChange, { instancePath: instancePath + "/packageSemanticIrChange", parentData: data, parentDataProperty: "packageSemanticIrChange", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate87.errors : vErrors.concat(validate87.errors);
        errors = vErrors.length;
      }
    }
    if (data.profileChange !== void 0) {
      if (!validate87(data.profileChange, { instancePath: instancePath + "/profileChange", parentData: data, parentDataProperty: "profileChange", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate87.errors : vErrors.concat(validate87.errors);
        errors = vErrors.length;
      }
    }
    if (data.profileDigest !== void 0) {
      if (typeof data.profileDigest !== "string") {
        const err36 = { instancePath: instancePath + "/profileDigest", schemaPath: "#/properties/profileDigest/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err36];
        } else {
          vErrors.push(err36);
        }
        errors++;
      }
    }
    if (data.projectId !== void 0) {
      if (typeof data.projectId !== "string") {
        const err37 = { instancePath: instancePath + "/projectId", schemaPath: "#/properties/projectId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err37];
        } else {
          vErrors.push(err37);
        }
        errors++;
      }
    }
    if (data.selectedTopologyChange !== void 0) {
      if (!validate87(data.selectedTopologyChange, { instancePath: instancePath + "/selectedTopologyChange", parentData: data, parentDataProperty: "selectedTopologyChange", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate87.errors : vErrors.concat(validate87.errors);
        errors = vErrors.length;
      }
    }
    if (data.unaffectedCount !== void 0) {
      let data18 = data.unaffectedCount;
      if (!(typeof data18 == "number" && (!(data18 % 1) && !isNaN(data18)))) {
        const err38 = { instancePath: instancePath + "/unaffectedCount", schemaPath: "#/properties/unaffectedCount/type", keyword: "type", params: { type: "integer" }, message: "must be integer" };
        if (vErrors === null) {
          vErrors = [err38];
        } else {
          vErrors.push(err38);
        }
        errors++;
      }
      if (typeof data18 == "number") {
        if (data18 < 0 || isNaN(data18)) {
          const err39 = { instancePath: instancePath + "/unaffectedCount", schemaPath: "#/properties/unaffectedCount/minimum", keyword: "minimum", params: { comparison: ">=", limit: 0 }, message: "must be >= 0" };
          if (vErrors === null) {
            vErrors = [err39];
          } else {
            vErrors.push(err39);
          }
          errors++;
        }
        if (!formats4.validate(data18)) {
          const err40 = { instancePath: instancePath + "/unaffectedCount", schemaPath: "#/properties/unaffectedCount/format", keyword: "format", params: { format: "uint32" }, message: 'must match format "uint32"' };
          if (vErrors === null) {
            vErrors = [err40];
          } else {
            vErrors.push(err40);
          }
          errors++;
        }
      }
    }
    if (data.workspaceId !== void 0) {
      if (typeof data.workspaceId !== "string") {
        const err41 = { instancePath: instancePath + "/workspaceId", schemaPath: "#/properties/workspaceId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err41];
        } else {
          vErrors.push(err41);
        }
        errors++;
      }
    }
  } else {
    const err42 = { instancePath, schemaPath: "#/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err42];
    } else {
      vErrors.push(err42);
    }
    errors++;
  }
  validate86.errors = vErrors;
  return errors === 0;
}
validate86.evaluated = { "props": true, "dynamicProps": false, "dynamicItems": false };
var schema23 = { "description": "One bounded page of exact comparison roots.", "properties": { "entries": { "items": { "$ref": "#/$defs/ChangeRoot" }, "type": "array" }, "nextOffset": { "format": "uint32", "minimum": 0, "type": ["integer", "null"] } }, "required": ["entries", "nextOffset"], "type": "object" };
function validate62(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate62.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  const _errs0 = errors;
  let valid0 = false;
  let passing0 = null;
  const _errs1 = errors;
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.kind === void 0) {
      const err0 = { instancePath, schemaPath: "#/oneOf/0/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
      if (vErrors === null) {
        vErrors = [err0];
      } else {
        vErrors.push(err0);
      }
      errors++;
    }
    if (data.sourcePath === void 0) {
      const err1 = { instancePath, schemaPath: "#/oneOf/0/required", keyword: "required", params: { missingProperty: "sourcePath" }, message: "must have required property 'sourcePath'" };
      if (vErrors === null) {
        vErrors = [err1];
      } else {
        vErrors.push(err1);
      }
      errors++;
    }
    if (data.sourceDigest === void 0) {
      const err2 = { instancePath, schemaPath: "#/oneOf/0/required", keyword: "required", params: { missingProperty: "sourceDigest" }, message: "must have required property 'sourceDigest'" };
      if (vErrors === null) {
        vErrors = [err2];
      } else {
        vErrors.push(err2);
      }
      errors++;
    }
    if (data.byteLen === void 0) {
      const err3 = { instancePath, schemaPath: "#/oneOf/0/required", keyword: "required", params: { missingProperty: "byteLen" }, message: "must have required property 'byteLen'" };
      if (vErrors === null) {
        vErrors = [err3];
      } else {
        vErrors.push(err3);
      }
      errors++;
    }
    if (data.byteLen !== void 0) {
      let data0 = data.byteLen;
      if (!(typeof data0 == "number" && (!(data0 % 1) && !isNaN(data0)))) {
        const err4 = { instancePath: instancePath + "/byteLen", schemaPath: "#/oneOf/0/properties/byteLen/type", keyword: "type", params: { type: "integer" }, message: "must be integer" };
        if (vErrors === null) {
          vErrors = [err4];
        } else {
          vErrors.push(err4);
        }
        errors++;
      }
      if (typeof data0 == "number") {
        if (data0 < 0 || isNaN(data0)) {
          const err5 = { instancePath: instancePath + "/byteLen", schemaPath: "#/oneOf/0/properties/byteLen/minimum", keyword: "minimum", params: { comparison: ">=", limit: 0 }, message: "must be >= 0" };
          if (vErrors === null) {
            vErrors = [err5];
          } else {
            vErrors.push(err5);
          }
          errors++;
        }
        if (!formats4.validate(data0)) {
          const err6 = { instancePath: instancePath + "/byteLen", schemaPath: "#/oneOf/0/properties/byteLen/format", keyword: "format", params: { format: "uint32" }, message: 'must match format "uint32"' };
          if (vErrors === null) {
            vErrors = [err6];
          } else {
            vErrors.push(err6);
          }
          errors++;
        }
      }
    }
    if (data.kind !== void 0) {
      let data1 = data.kind;
      if (typeof data1 !== "string") {
        const err7 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/0/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err7];
        } else {
          vErrors.push(err7);
        }
        errors++;
      }
      if ("source-content" !== data1) {
        const err8 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/0/properties/kind/const", keyword: "const", params: { allowedValue: "source-content" }, message: "must be equal to constant" };
        if (vErrors === null) {
          vErrors = [err8];
        } else {
          vErrors.push(err8);
        }
        errors++;
      }
    }
    if (data.sourceDigest !== void 0) {
      if (typeof data.sourceDigest !== "string") {
        const err9 = { instancePath: instancePath + "/sourceDigest", schemaPath: "#/oneOf/0/properties/sourceDigest/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err9];
        } else {
          vErrors.push(err9);
        }
        errors++;
      }
    }
    if (data.sourcePath !== void 0) {
      if (typeof data.sourcePath !== "string") {
        const err10 = { instancePath: instancePath + "/sourcePath", schemaPath: "#/oneOf/0/properties/sourcePath/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err10];
        } else {
          vErrors.push(err10);
        }
        errors++;
      }
    }
  } else {
    const err11 = { instancePath, schemaPath: "#/oneOf/0/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err11];
    } else {
      vErrors.push(err11);
    }
    errors++;
  }
  var _valid0 = _errs1 === errors;
  if (_valid0) {
    valid0 = true;
    passing0 = 0;
    var props0 = {};
    props0.byteLen = true;
    props0.kind = true;
    props0.sourceDigest = true;
    props0.sourcePath = true;
  }
  const _errs11 = errors;
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.kind === void 0) {
      const err12 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
      if (vErrors === null) {
        vErrors = [err12];
      } else {
        vErrors.push(err12);
      }
      errors++;
    }
    if (data.componentDigest === void 0) {
      const err13 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "componentDigest" }, message: "must have required property 'componentDigest'" };
      if (vErrors === null) {
        vErrors = [err13];
      } else {
        vErrors.push(err13);
      }
      errors++;
    }
    if (data.componentDigest !== void 0) {
      if (typeof data.componentDigest !== "string") {
        const err14 = { instancePath: instancePath + "/componentDigest", schemaPath: "#/oneOf/1/properties/componentDigest/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err14];
        } else {
          vErrors.push(err14);
        }
        errors++;
      }
    }
    if (data.kind !== void 0) {
      let data5 = data.kind;
      if (typeof data5 !== "string") {
        const err15 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/1/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err15];
        } else {
          vErrors.push(err15);
        }
        errors++;
      }
      if ("component" !== data5) {
        const err16 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/1/properties/kind/const", keyword: "const", params: { allowedValue: "component" }, message: "must be equal to constant" };
        if (vErrors === null) {
          vErrors = [err16];
        } else {
          vErrors.push(err16);
        }
        errors++;
      }
    }
  } else {
    const err17 = { instancePath, schemaPath: "#/oneOf/1/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err17];
    } else {
      vErrors.push(err17);
    }
    errors++;
  }
  var _valid0 = _errs11 === errors;
  if (_valid0 && valid0) {
    valid0 = false;
    passing0 = [passing0, 1];
  } else {
    if (_valid0) {
      valid0 = true;
      passing0 = 1;
      if (props0 !== true) {
        props0 = props0 || {};
        props0.componentDigest = true;
        props0.kind = true;
      }
    }
    const _errs17 = errors;
    if (data && typeof data == "object" && !Array.isArray(data)) {
      if (data.kind === void 0) {
        const err18 = { instancePath, schemaPath: "#/oneOf/2/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
        if (vErrors === null) {
          vErrors = [err18];
        } else {
          vErrors.push(err18);
        }
        errors++;
      }
      if (data.moduleDigest === void 0) {
        const err19 = { instancePath, schemaPath: "#/oneOf/2/required", keyword: "required", params: { missingProperty: "moduleDigest" }, message: "must have required property 'moduleDigest'" };
        if (vErrors === null) {
          vErrors = [err19];
        } else {
          vErrors.push(err19);
        }
        errors++;
      }
      if (data.kind !== void 0) {
        let data6 = data.kind;
        if (typeof data6 !== "string") {
          const err20 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/2/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
          if (vErrors === null) {
            vErrors = [err20];
          } else {
            vErrors.push(err20);
          }
          errors++;
        }
        if ("module" !== data6) {
          const err21 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/2/properties/kind/const", keyword: "const", params: { allowedValue: "module" }, message: "must be equal to constant" };
          if (vErrors === null) {
            vErrors = [err21];
          } else {
            vErrors.push(err21);
          }
          errors++;
        }
      }
      if (data.moduleDigest !== void 0) {
        if (typeof data.moduleDigest !== "string") {
          const err22 = { instancePath: instancePath + "/moduleDigest", schemaPath: "#/oneOf/2/properties/moduleDigest/type", keyword: "type", params: { type: "string" }, message: "must be string" };
          if (vErrors === null) {
            vErrors = [err22];
          } else {
            vErrors.push(err22);
          }
          errors++;
        }
      }
    } else {
      const err23 = { instancePath, schemaPath: "#/oneOf/2/type", keyword: "type", params: { type: "object" }, message: "must be object" };
      if (vErrors === null) {
        vErrors = [err23];
      } else {
        vErrors.push(err23);
      }
      errors++;
    }
    var _valid0 = _errs17 === errors;
    if (_valid0 && valid0) {
      valid0 = false;
      passing0 = [passing0, 2];
    } else {
      if (_valid0) {
        valid0 = true;
        passing0 = 2;
        if (props0 !== true) {
          props0 = props0 || {};
          props0.kind = true;
          props0.moduleDigest = true;
        }
      }
      const _errs23 = errors;
      if (data && typeof data == "object" && !Array.isArray(data)) {
        if (data.kind === void 0) {
          const err24 = { instancePath, schemaPath: "#/oneOf/3/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
          if (vErrors === null) {
            vErrors = [err24];
          } else {
            vErrors.push(err24);
          }
          errors++;
        }
        if (data.valueDigest === void 0) {
          const err25 = { instancePath, schemaPath: "#/oneOf/3/required", keyword: "required", params: { missingProperty: "valueDigest" }, message: "must have required property 'valueDigest'" };
          if (vErrors === null) {
            vErrors = [err25];
          } else {
            vErrors.push(err25);
          }
          errors++;
        }
        if (data.kind !== void 0) {
          let data8 = data.kind;
          if (typeof data8 !== "string") {
            const err26 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/3/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
            if (vErrors === null) {
              vErrors = [err26];
            } else {
              vErrors.push(err26);
            }
            errors++;
          }
          if ("declaration" !== data8) {
            const err27 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/3/properties/kind/const", keyword: "const", params: { allowedValue: "declaration" }, message: "must be equal to constant" };
            if (vErrors === null) {
              vErrors = [err27];
            } else {
              vErrors.push(err27);
            }
            errors++;
          }
        }
        if (data.valueDigest !== void 0) {
          if (typeof data.valueDigest !== "string") {
            const err28 = { instancePath: instancePath + "/valueDigest", schemaPath: "#/oneOf/3/properties/valueDigest/type", keyword: "type", params: { type: "string" }, message: "must be string" };
            if (vErrors === null) {
              vErrors = [err28];
            } else {
              vErrors.push(err28);
            }
            errors++;
          }
        }
      } else {
        const err29 = { instancePath, schemaPath: "#/oneOf/3/type", keyword: "type", params: { type: "object" }, message: "must be object" };
        if (vErrors === null) {
          vErrors = [err29];
        } else {
          vErrors.push(err29);
        }
        errors++;
      }
      var _valid0 = _errs23 === errors;
      if (_valid0 && valid0) {
        valid0 = false;
        passing0 = [passing0, 3];
      } else {
        if (_valid0) {
          valid0 = true;
          passing0 = 3;
          if (props0 !== true) {
            props0 = props0 || {};
            props0.kind = true;
            props0.valueDigest = true;
          }
        }
        const _errs29 = errors;
        if (data && typeof data == "object" && !Array.isArray(data)) {
          if (data.kind === void 0) {
            const err30 = { instancePath, schemaPath: "#/oneOf/4/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
            if (vErrors === null) {
              vErrors = [err30];
            } else {
              vErrors.push(err30);
            }
            errors++;
          }
          if (data.surfaceDigest === void 0) {
            const err31 = { instancePath, schemaPath: "#/oneOf/4/required", keyword: "required", params: { missingProperty: "surfaceDigest" }, message: "must have required property 'surfaceDigest'" };
            if (vErrors === null) {
              vErrors = [err31];
            } else {
              vErrors.push(err31);
            }
            errors++;
          }
          if (data.kind !== void 0) {
            let data10 = data.kind;
            if (typeof data10 !== "string") {
              const err32 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/4/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
              if (vErrors === null) {
                vErrors = [err32];
              } else {
                vErrors.push(err32);
              }
              errors++;
            }
            if ("import-surface" !== data10) {
              const err33 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/4/properties/kind/const", keyword: "const", params: { allowedValue: "import-surface" }, message: "must be equal to constant" };
              if (vErrors === null) {
                vErrors = [err33];
              } else {
                vErrors.push(err33);
              }
              errors++;
            }
          }
          if (data.surfaceDigest !== void 0) {
            if (typeof data.surfaceDigest !== "string") {
              const err34 = { instancePath: instancePath + "/surfaceDigest", schemaPath: "#/oneOf/4/properties/surfaceDigest/type", keyword: "type", params: { type: "string" }, message: "must be string" };
              if (vErrors === null) {
                vErrors = [err34];
              } else {
                vErrors.push(err34);
              }
              errors++;
            }
          }
        } else {
          const err35 = { instancePath, schemaPath: "#/oneOf/4/type", keyword: "type", params: { type: "object" }, message: "must be object" };
          if (vErrors === null) {
            vErrors = [err35];
          } else {
            vErrors.push(err35);
          }
          errors++;
        }
        var _valid0 = _errs29 === errors;
        if (_valid0 && valid0) {
          valid0 = false;
          passing0 = [passing0, 4];
        } else {
          if (_valid0) {
            valid0 = true;
            passing0 = 4;
            if (props0 !== true) {
              props0 = props0 || {};
              props0.kind = true;
              props0.surfaceDigest = true;
            }
          }
          const _errs35 = errors;
          if (data && typeof data == "object" && !Array.isArray(data)) {
            if (data.kind === void 0) {
              const err36 = { instancePath, schemaPath: "#/oneOf/5/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
              if (vErrors === null) {
                vErrors = [err36];
              } else {
                vErrors.push(err36);
              }
              errors++;
            }
            if (data.valueDigest === void 0) {
              const err37 = { instancePath, schemaPath: "#/oneOf/5/required", keyword: "required", params: { missingProperty: "valueDigest" }, message: "must have required property 'valueDigest'" };
              if (vErrors === null) {
                vErrors = [err37];
              } else {
                vErrors.push(err37);
              }
              errors++;
            }
            if (data.kind !== void 0) {
              let data12 = data.kind;
              if (typeof data12 !== "string") {
                const err38 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/5/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                if (vErrors === null) {
                  vErrors = [err38];
                } else {
                  vErrors.push(err38);
                }
                errors++;
              }
              if ("extension" !== data12) {
                const err39 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/5/properties/kind/const", keyword: "const", params: { allowedValue: "extension" }, message: "must be equal to constant" };
                if (vErrors === null) {
                  vErrors = [err39];
                } else {
                  vErrors.push(err39);
                }
                errors++;
              }
            }
            if (data.valueDigest !== void 0) {
              if (typeof data.valueDigest !== "string") {
                const err40 = { instancePath: instancePath + "/valueDigest", schemaPath: "#/oneOf/5/properties/valueDigest/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                if (vErrors === null) {
                  vErrors = [err40];
                } else {
                  vErrors.push(err40);
                }
                errors++;
              }
            }
          } else {
            const err41 = { instancePath, schemaPath: "#/oneOf/5/type", keyword: "type", params: { type: "object" }, message: "must be object" };
            if (vErrors === null) {
              vErrors = [err41];
            } else {
              vErrors.push(err41);
            }
            errors++;
          }
          var _valid0 = _errs35 === errors;
          if (_valid0 && valid0) {
            valid0 = false;
            passing0 = [passing0, 5];
          } else {
            if (_valid0) {
              valid0 = true;
              passing0 = 5;
              if (props0 !== true) {
                props0 = props0 || {};
                props0.kind = true;
                props0.valueDigest = true;
              }
            }
            const _errs41 = errors;
            if (data && typeof data == "object" && !Array.isArray(data)) {
              if (data.kind === void 0) {
                const err42 = { instancePath, schemaPath: "#/oneOf/6/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
                if (vErrors === null) {
                  vErrors = [err42];
                } else {
                  vErrors.push(err42);
                }
                errors++;
              }
              if (data.valueDigest === void 0) {
                const err43 = { instancePath, schemaPath: "#/oneOf/6/required", keyword: "required", params: { missingProperty: "valueDigest" }, message: "must have required property 'valueDigest'" };
                if (vErrors === null) {
                  vErrors = [err43];
                } else {
                  vErrors.push(err43);
                }
                errors++;
              }
              if (data.kind !== void 0) {
                let data14 = data.kind;
                if (typeof data14 !== "string") {
                  const err44 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/6/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                  if (vErrors === null) {
                    vErrors = [err44];
                  } else {
                    vErrors.push(err44);
                  }
                  errors++;
                }
                if ("effective-policy" !== data14) {
                  const err45 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/6/properties/kind/const", keyword: "const", params: { allowedValue: "effective-policy" }, message: "must be equal to constant" };
                  if (vErrors === null) {
                    vErrors = [err45];
                  } else {
                    vErrors.push(err45);
                  }
                  errors++;
                }
              }
              if (data.valueDigest !== void 0) {
                if (typeof data.valueDigest !== "string") {
                  const err46 = { instancePath: instancePath + "/valueDigest", schemaPath: "#/oneOf/6/properties/valueDigest/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                  if (vErrors === null) {
                    vErrors = [err46];
                  } else {
                    vErrors.push(err46);
                  }
                  errors++;
                }
              }
            } else {
              const err47 = { instancePath, schemaPath: "#/oneOf/6/type", keyword: "type", params: { type: "object" }, message: "must be object" };
              if (vErrors === null) {
                vErrors = [err47];
              } else {
                vErrors.push(err47);
              }
              errors++;
            }
            var _valid0 = _errs41 === errors;
            if (_valid0 && valid0) {
              valid0 = false;
              passing0 = [passing0, 6];
            } else {
              if (_valid0) {
                valid0 = true;
                passing0 = 6;
                if (props0 !== true) {
                  props0 = props0 || {};
                  props0.kind = true;
                  props0.valueDigest = true;
                }
              }
              const _errs47 = errors;
              if (data && typeof data == "object" && !Array.isArray(data)) {
                if (data.kind === void 0) {
                  const err48 = { instancePath, schemaPath: "#/oneOf/7/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
                  if (vErrors === null) {
                    vErrors = [err48];
                  } else {
                    vErrors.push(err48);
                  }
                  errors++;
                }
                if (data.closureDigest === void 0) {
                  const err49 = { instancePath, schemaPath: "#/oneOf/7/required", keyword: "required", params: { missingProperty: "closureDigest" }, message: "must have required property 'closureDigest'" };
                  if (vErrors === null) {
                    vErrors = [err49];
                  } else {
                    vErrors.push(err49);
                  }
                  errors++;
                }
                if (data.closureDigest !== void 0) {
                  if (typeof data.closureDigest !== "string") {
                    const err50 = { instancePath: instancePath + "/closureDigest", schemaPath: "#/oneOf/7/properties/closureDigest/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                    if (vErrors === null) {
                      vErrors = [err50];
                    } else {
                      vErrors.push(err50);
                    }
                    errors++;
                  }
                }
                if (data.kind !== void 0) {
                  let data17 = data.kind;
                  if (typeof data17 !== "string") {
                    const err51 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/7/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                    if (vErrors === null) {
                      vErrors = [err51];
                    } else {
                      vErrors.push(err51);
                    }
                    errors++;
                  }
                  if ("responsibility-closure" !== data17) {
                    const err52 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/7/properties/kind/const", keyword: "const", params: { allowedValue: "responsibility-closure" }, message: "must be equal to constant" };
                    if (vErrors === null) {
                      vErrors = [err52];
                    } else {
                      vErrors.push(err52);
                    }
                    errors++;
                  }
                }
              } else {
                const err53 = { instancePath, schemaPath: "#/oneOf/7/type", keyword: "type", params: { type: "object" }, message: "must be object" };
                if (vErrors === null) {
                  vErrors = [err53];
                } else {
                  vErrors.push(err53);
                }
                errors++;
              }
              var _valid0 = _errs47 === errors;
              if (_valid0 && valid0) {
                valid0 = false;
                passing0 = [passing0, 7];
              } else {
                if (_valid0) {
                  valid0 = true;
                  passing0 = 7;
                  if (props0 !== true) {
                    props0 = props0 || {};
                    props0.closureDigest = true;
                    props0.kind = true;
                  }
                }
                const _errs53 = errors;
                if (data && typeof data == "object" && !Array.isArray(data)) {
                  if (data.kind === void 0) {
                    const err54 = { instancePath, schemaPath: "#/oneOf/8/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
                    if (vErrors === null) {
                      vErrors = [err54];
                    } else {
                      vErrors.push(err54);
                    }
                    errors++;
                  }
                  if (data.closureDigest === void 0) {
                    const err55 = { instancePath, schemaPath: "#/oneOf/8/required", keyword: "required", params: { missingProperty: "closureDigest" }, message: "must have required property 'closureDigest'" };
                    if (vErrors === null) {
                      vErrors = [err55];
                    } else {
                      vErrors.push(err55);
                    }
                    errors++;
                  }
                  if (data.closureDigest !== void 0) {
                    if (typeof data.closureDigest !== "string") {
                      const err56 = { instancePath: instancePath + "/closureDigest", schemaPath: "#/oneOf/8/properties/closureDigest/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                      if (vErrors === null) {
                        vErrors = [err56];
                      } else {
                        vErrors.push(err56);
                      }
                      errors++;
                    }
                  }
                  if (data.kind !== void 0) {
                    let data19 = data.kind;
                    if (typeof data19 !== "string") {
                      const err57 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/8/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                      if (vErrors === null) {
                        vErrors = [err57];
                      } else {
                        vErrors.push(err57);
                      }
                      errors++;
                    }
                    if ("organizational-translation-closure" !== data19) {
                      const err58 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/8/properties/kind/const", keyword: "const", params: { allowedValue: "organizational-translation-closure" }, message: "must be equal to constant" };
                      if (vErrors === null) {
                        vErrors = [err58];
                      } else {
                        vErrors.push(err58);
                      }
                      errors++;
                    }
                  }
                } else {
                  const err59 = { instancePath, schemaPath: "#/oneOf/8/type", keyword: "type", params: { type: "object" }, message: "must be object" };
                  if (vErrors === null) {
                    vErrors = [err59];
                  } else {
                    vErrors.push(err59);
                  }
                  errors++;
                }
                var _valid0 = _errs53 === errors;
                if (_valid0 && valid0) {
                  valid0 = false;
                  passing0 = [passing0, 8];
                } else {
                  if (_valid0) {
                    valid0 = true;
                    passing0 = 8;
                    if (props0 !== true) {
                      props0 = props0 || {};
                      props0.closureDigest = true;
                      props0.kind = true;
                    }
                  }
                  const _errs59 = errors;
                  if (data && typeof data == "object" && !Array.isArray(data)) {
                    if (data.kind === void 0) {
                      const err60 = { instancePath, schemaPath: "#/oneOf/9/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
                      if (vErrors === null) {
                        vErrors = [err60];
                      } else {
                        vErrors.push(err60);
                      }
                      errors++;
                    }
                    if (data.sourceDigest === void 0) {
                      const err61 = { instancePath, schemaPath: "#/oneOf/9/required", keyword: "required", params: { missingProperty: "sourceDigest" }, message: "must have required property 'sourceDigest'" };
                      if (vErrors === null) {
                        vErrors = [err61];
                      } else {
                        vErrors.push(err61);
                      }
                      errors++;
                    }
                    if (data.kind !== void 0) {
                      let data20 = data.kind;
                      if (typeof data20 !== "string") {
                        const err62 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/9/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                        if (vErrors === null) {
                          vErrors = [err62];
                        } else {
                          vErrors.push(err62);
                        }
                        errors++;
                      }
                      if ("experience-composition" !== data20) {
                        const err63 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/9/properties/kind/const", keyword: "const", params: { allowedValue: "experience-composition" }, message: "must be equal to constant" };
                        if (vErrors === null) {
                          vErrors = [err63];
                        } else {
                          vErrors.push(err63);
                        }
                        errors++;
                      }
                    }
                    if (data.sourceDigest !== void 0) {
                      if (typeof data.sourceDigest !== "string") {
                        const err64 = { instancePath: instancePath + "/sourceDigest", schemaPath: "#/oneOf/9/properties/sourceDigest/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                        if (vErrors === null) {
                          vErrors = [err64];
                        } else {
                          vErrors.push(err64);
                        }
                        errors++;
                      }
                    }
                  } else {
                    const err65 = { instancePath, schemaPath: "#/oneOf/9/type", keyword: "type", params: { type: "object" }, message: "must be object" };
                    if (vErrors === null) {
                      vErrors = [err65];
                    } else {
                      vErrors.push(err65);
                    }
                    errors++;
                  }
                  var _valid0 = _errs59 === errors;
                  if (_valid0 && valid0) {
                    valid0 = false;
                    passing0 = [passing0, 9];
                  } else {
                    if (_valid0) {
                      valid0 = true;
                      passing0 = 9;
                      if (props0 !== true) {
                        props0 = props0 || {};
                        props0.kind = true;
                        props0.sourceDigest = true;
                      }
                    }
                    const _errs65 = errors;
                    if (data && typeof data == "object" && !Array.isArray(data)) {
                      if (data.kind === void 0) {
                        const err66 = { instancePath, schemaPath: "#/oneOf/10/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
                        if (vErrors === null) {
                          vErrors = [err66];
                        } else {
                          vErrors.push(err66);
                        }
                        errors++;
                      }
                      if (data.valueDigest === void 0) {
                        const err67 = { instancePath, schemaPath: "#/oneOf/10/required", keyword: "required", params: { missingProperty: "valueDigest" }, message: "must have required property 'valueDigest'" };
                        if (vErrors === null) {
                          vErrors = [err67];
                        } else {
                          vErrors.push(err67);
                        }
                        errors++;
                      }
                      if (data.kind !== void 0) {
                        let data22 = data.kind;
                        if (typeof data22 !== "string") {
                          const err68 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/10/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                          if (vErrors === null) {
                            vErrors = [err68];
                          } else {
                            vErrors.push(err68);
                          }
                          errors++;
                        }
                        if ("resource" !== data22) {
                          const err69 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/10/properties/kind/const", keyword: "const", params: { allowedValue: "resource" }, message: "must be equal to constant" };
                          if (vErrors === null) {
                            vErrors = [err69];
                          } else {
                            vErrors.push(err69);
                          }
                          errors++;
                        }
                      }
                      if (data.valueDigest !== void 0) {
                        if (typeof data.valueDigest !== "string") {
                          const err70 = { instancePath: instancePath + "/valueDigest", schemaPath: "#/oneOf/10/properties/valueDigest/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                          if (vErrors === null) {
                            vErrors = [err70];
                          } else {
                            vErrors.push(err70);
                          }
                          errors++;
                        }
                      }
                    } else {
                      const err71 = { instancePath, schemaPath: "#/oneOf/10/type", keyword: "type", params: { type: "object" }, message: "must be object" };
                      if (vErrors === null) {
                        vErrors = [err71];
                      } else {
                        vErrors.push(err71);
                      }
                      errors++;
                    }
                    var _valid0 = _errs65 === errors;
                    if (_valid0 && valid0) {
                      valid0 = false;
                      passing0 = [passing0, 10];
                    } else {
                      if (_valid0) {
                        valid0 = true;
                        passing0 = 10;
                        if (props0 !== true) {
                          props0 = props0 || {};
                          props0.kind = true;
                          props0.valueDigest = true;
                        }
                      }
                      const _errs71 = errors;
                      if (data && typeof data == "object" && !Array.isArray(data)) {
                        if (data.kind === void 0) {
                          const err72 = { instancePath, schemaPath: "#/oneOf/11/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
                          if (vErrors === null) {
                            vErrors = [err72];
                          } else {
                            vErrors.push(err72);
                          }
                          errors++;
                        }
                        if (data.resolutionDigest === void 0) {
                          const err73 = { instancePath, schemaPath: "#/oneOf/11/required", keyword: "required", params: { missingProperty: "resolutionDigest" }, message: "must have required property 'resolutionDigest'" };
                          if (vErrors === null) {
                            vErrors = [err73];
                          } else {
                            vErrors.push(err73);
                          }
                          errors++;
                        }
                        if (data.kind !== void 0) {
                          let data24 = data.kind;
                          if (typeof data24 !== "string") {
                            const err74 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/11/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                            if (vErrors === null) {
                              vErrors = [err74];
                            } else {
                              vErrors.push(err74);
                            }
                            errors++;
                          }
                          if ("resolution-lock" !== data24) {
                            const err75 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/11/properties/kind/const", keyword: "const", params: { allowedValue: "resolution-lock" }, message: "must be equal to constant" };
                            if (vErrors === null) {
                              vErrors = [err75];
                            } else {
                              vErrors.push(err75);
                            }
                            errors++;
                          }
                        }
                        if (data.resolutionDigest !== void 0) {
                          if (typeof data.resolutionDigest !== "string") {
                            const err76 = { instancePath: instancePath + "/resolutionDigest", schemaPath: "#/oneOf/11/properties/resolutionDigest/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                            if (vErrors === null) {
                              vErrors = [err76];
                            } else {
                              vErrors.push(err76);
                            }
                            errors++;
                          }
                        }
                      } else {
                        const err77 = { instancePath, schemaPath: "#/oneOf/11/type", keyword: "type", params: { type: "object" }, message: "must be object" };
                        if (vErrors === null) {
                          vErrors = [err77];
                        } else {
                          vErrors.push(err77);
                        }
                        errors++;
                      }
                      var _valid0 = _errs71 === errors;
                      if (_valid0 && valid0) {
                        valid0 = false;
                        passing0 = [passing0, 11];
                      } else {
                        if (_valid0) {
                          valid0 = true;
                          passing0 = 11;
                          if (props0 !== true) {
                            props0 = props0 || {};
                            props0.kind = true;
                            props0.resolutionDigest = true;
                          }
                        }
                        const _errs77 = errors;
                        if (data && typeof data == "object" && !Array.isArray(data)) {
                          if (data.kind === void 0) {
                            const err78 = { instancePath, schemaPath: "#/oneOf/12/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
                            if (vErrors === null) {
                              vErrors = [err78];
                            } else {
                              vErrors.push(err78);
                            }
                            errors++;
                          }
                          if (data.universeDigest === void 0) {
                            const err79 = { instancePath, schemaPath: "#/oneOf/12/required", keyword: "required", params: { missingProperty: "universeDigest" }, message: "must have required property 'universeDigest'" };
                            if (vErrors === null) {
                              vErrors = [err79];
                            } else {
                              vErrors.push(err79);
                            }
                            errors++;
                          }
                          if (data.kind !== void 0) {
                            let data26 = data.kind;
                            if (typeof data26 !== "string") {
                              const err80 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/12/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                              if (vErrors === null) {
                                vErrors = [err80];
                              } else {
                                vErrors.push(err80);
                              }
                              errors++;
                            }
                            if ("universe-lock" !== data26) {
                              const err81 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/12/properties/kind/const", keyword: "const", params: { allowedValue: "universe-lock" }, message: "must be equal to constant" };
                              if (vErrors === null) {
                                vErrors = [err81];
                              } else {
                                vErrors.push(err81);
                              }
                              errors++;
                            }
                          }
                          if (data.universeDigest !== void 0) {
                            if (typeof data.universeDigest !== "string") {
                              const err82 = { instancePath: instancePath + "/universeDigest", schemaPath: "#/oneOf/12/properties/universeDigest/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                              if (vErrors === null) {
                                vErrors = [err82];
                              } else {
                                vErrors.push(err82);
                              }
                              errors++;
                            }
                          }
                        } else {
                          const err83 = { instancePath, schemaPath: "#/oneOf/12/type", keyword: "type", params: { type: "object" }, message: "must be object" };
                          if (vErrors === null) {
                            vErrors = [err83];
                          } else {
                            vErrors.push(err83);
                          }
                          errors++;
                        }
                        var _valid0 = _errs77 === errors;
                        if (_valid0 && valid0) {
                          valid0 = false;
                          passing0 = [passing0, 12];
                        } else {
                          if (_valid0) {
                            valid0 = true;
                            passing0 = 12;
                            if (props0 !== true) {
                              props0 = props0 || {};
                              props0.kind = true;
                              props0.universeDigest = true;
                            }
                          }
                          const _errs83 = errors;
                          if (data && typeof data == "object" && !Array.isArray(data)) {
                            if (data.kind === void 0) {
                              const err84 = { instancePath, schemaPath: "#/oneOf/13/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
                              if (vErrors === null) {
                                vErrors = [err84];
                              } else {
                                vErrors.push(err84);
                              }
                              errors++;
                            }
                            if (data.workspaceSemanticIrDigest === void 0) {
                              const err85 = { instancePath, schemaPath: "#/oneOf/13/required", keyword: "required", params: { missingProperty: "workspaceSemanticIrDigest" }, message: "must have required property 'workspaceSemanticIrDigest'" };
                              if (vErrors === null) {
                                vErrors = [err85];
                              } else {
                                vErrors.push(err85);
                              }
                              errors++;
                            }
                            if (data.kind !== void 0) {
                              let data28 = data.kind;
                              if (typeof data28 !== "string") {
                                const err86 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/13/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                                if (vErrors === null) {
                                  vErrors = [err86];
                                } else {
                                  vErrors.push(err86);
                                }
                                errors++;
                              }
                              if ("semantic-ir" !== data28) {
                                const err87 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/13/properties/kind/const", keyword: "const", params: { allowedValue: "semantic-ir" }, message: "must be equal to constant" };
                                if (vErrors === null) {
                                  vErrors = [err87];
                                } else {
                                  vErrors.push(err87);
                                }
                                errors++;
                              }
                            }
                            if (data.workspaceSemanticIrDigest !== void 0) {
                              if (typeof data.workspaceSemanticIrDigest !== "string") {
                                const err88 = { instancePath: instancePath + "/workspaceSemanticIrDigest", schemaPath: "#/oneOf/13/properties/workspaceSemanticIrDigest/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                                if (vErrors === null) {
                                  vErrors = [err88];
                                } else {
                                  vErrors.push(err88);
                                }
                                errors++;
                              }
                            }
                          } else {
                            const err89 = { instancePath, schemaPath: "#/oneOf/13/type", keyword: "type", params: { type: "object" }, message: "must be object" };
                            if (vErrors === null) {
                              vErrors = [err89];
                            } else {
                              vErrors.push(err89);
                            }
                            errors++;
                          }
                          var _valid0 = _errs83 === errors;
                          if (_valid0 && valid0) {
                            valid0 = false;
                            passing0 = [passing0, 13];
                          } else {
                            if (_valid0) {
                              valid0 = true;
                              passing0 = 13;
                              if (props0 !== true) {
                                props0 = props0 || {};
                                props0.kind = true;
                                props0.workspaceSemanticIrDigest = true;
                              }
                            }
                          }
                        }
                      }
                    }
                  }
                }
              }
            }
          }
        }
      }
    }
  }
  if (!valid0) {
    const err90 = { instancePath, schemaPath: "#/oneOf", keyword: "oneOf", params: { passingSchemas: passing0 }, message: "must match exactly one schema in oneOf" };
    if (vErrors === null) {
      vErrors = [err90];
    } else {
      vErrors.push(err90);
    }
    errors++;
  } else {
    errors = _errs0;
    if (vErrors !== null) {
      if (_errs0) {
        vErrors.length = _errs0;
      } else {
        vErrors = null;
      }
    }
  }
  validate62.errors = vErrors;
  evaluated0.props = props0;
  return errors === 0;
}
validate62.evaluated = { "dynamicProps": true, "dynamicItems": false };
function validate61(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate61.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  const _errs0 = errors;
  let valid0 = false;
  let passing0 = null;
  const _errs1 = errors;
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.kind === void 0) {
      const err0 = { instancePath, schemaPath: "#/oneOf/0/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
      if (vErrors === null) {
        vErrors = [err0];
      } else {
        vErrors.push(err0);
      }
      errors++;
    }
    if (data.current === void 0) {
      const err1 = { instancePath, schemaPath: "#/oneOf/0/required", keyword: "required", params: { missingProperty: "current" }, message: "must have required property 'current'" };
      if (vErrors === null) {
        vErrors = [err1];
      } else {
        vErrors.push(err1);
      }
      errors++;
    }
    if (data.current !== void 0) {
      if (!validate62(data.current, { instancePath: instancePath + "/current", parentData: data, parentDataProperty: "current", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate62.errors : vErrors.concat(validate62.errors);
        errors = vErrors.length;
      }
    }
    if (data.kind !== void 0) {
      let data1 = data.kind;
      if (typeof data1 !== "string") {
        const err2 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/0/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err2];
        } else {
          vErrors.push(err2);
        }
        errors++;
      }
      if ("added" !== data1) {
        const err3 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/0/properties/kind/const", keyword: "const", params: { allowedValue: "added" }, message: "must be equal to constant" };
        if (vErrors === null) {
          vErrors = [err3];
        } else {
          vErrors.push(err3);
        }
        errors++;
      }
    }
  } else {
    const err4 = { instancePath, schemaPath: "#/oneOf/0/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err4];
    } else {
      vErrors.push(err4);
    }
    errors++;
  }
  var _valid0 = _errs1 === errors;
  if (_valid0) {
    valid0 = true;
    passing0 = 0;
    var props1 = {};
    props1.current = true;
    props1.kind = true;
  }
  const _errs6 = errors;
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.kind === void 0) {
      const err5 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
      if (vErrors === null) {
        vErrors = [err5];
      } else {
        vErrors.push(err5);
      }
      errors++;
    }
    if (data.prior === void 0) {
      const err6 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "prior" }, message: "must have required property 'prior'" };
      if (vErrors === null) {
        vErrors = [err6];
      } else {
        vErrors.push(err6);
      }
      errors++;
    }
    if (data.current === void 0) {
      const err7 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "current" }, message: "must have required property 'current'" };
      if (vErrors === null) {
        vErrors = [err7];
      } else {
        vErrors.push(err7);
      }
      errors++;
    }
    if (data.current !== void 0) {
      if (!validate62(data.current, { instancePath: instancePath + "/current", parentData: data, parentDataProperty: "current", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate62.errors : vErrors.concat(validate62.errors);
        errors = vErrors.length;
      }
    }
    if (data.kind !== void 0) {
      let data3 = data.kind;
      if (typeof data3 !== "string") {
        const err8 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/1/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err8];
        } else {
          vErrors.push(err8);
        }
        errors++;
      }
      if ("modified" !== data3) {
        const err9 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/1/properties/kind/const", keyword: "const", params: { allowedValue: "modified" }, message: "must be equal to constant" };
        if (vErrors === null) {
          vErrors = [err9];
        } else {
          vErrors.push(err9);
        }
        errors++;
      }
    }
    if (data.prior !== void 0) {
      if (!validate62(data.prior, { instancePath: instancePath + "/prior", parentData: data, parentDataProperty: "prior", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate62.errors : vErrors.concat(validate62.errors);
        errors = vErrors.length;
      }
    }
  } else {
    const err10 = { instancePath, schemaPath: "#/oneOf/1/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err10];
    } else {
      vErrors.push(err10);
    }
    errors++;
  }
  var _valid0 = _errs6 === errors;
  if (_valid0 && valid0) {
    valid0 = false;
    passing0 = [passing0, 1];
  } else {
    if (_valid0) {
      valid0 = true;
      passing0 = 1;
      if (props1 !== true) {
        props1 = props1 || {};
        props1.current = true;
        props1.kind = true;
        props1.prior = true;
      }
    }
    const _errs12 = errors;
    if (data && typeof data == "object" && !Array.isArray(data)) {
      if (data.kind === void 0) {
        const err11 = { instancePath, schemaPath: "#/oneOf/2/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
        if (vErrors === null) {
          vErrors = [err11];
        } else {
          vErrors.push(err11);
        }
        errors++;
      }
      if (data.prior === void 0) {
        const err12 = { instancePath, schemaPath: "#/oneOf/2/required", keyword: "required", params: { missingProperty: "prior" }, message: "must have required property 'prior'" };
        if (vErrors === null) {
          vErrors = [err12];
        } else {
          vErrors.push(err12);
        }
        errors++;
      }
      if (data.kind !== void 0) {
        let data5 = data.kind;
        if (typeof data5 !== "string") {
          const err13 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/2/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
          if (vErrors === null) {
            vErrors = [err13];
          } else {
            vErrors.push(err13);
          }
          errors++;
        }
        if ("removed" !== data5) {
          const err14 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/2/properties/kind/const", keyword: "const", params: { allowedValue: "removed" }, message: "must be equal to constant" };
          if (vErrors === null) {
            vErrors = [err14];
          } else {
            vErrors.push(err14);
          }
          errors++;
        }
      }
      if (data.prior !== void 0) {
        if (!validate62(data.prior, { instancePath: instancePath + "/prior", parentData: data, parentDataProperty: "prior", rootData, dynamicAnchors })) {
          vErrors = vErrors === null ? validate62.errors : vErrors.concat(validate62.errors);
          errors = vErrors.length;
        }
      }
    } else {
      const err15 = { instancePath, schemaPath: "#/oneOf/2/type", keyword: "type", params: { type: "object" }, message: "must be object" };
      if (vErrors === null) {
        vErrors = [err15];
      } else {
        vErrors.push(err15);
      }
      errors++;
    }
    var _valid0 = _errs12 === errors;
    if (_valid0 && valid0) {
      valid0 = false;
      passing0 = [passing0, 2];
    } else {
      if (_valid0) {
        valid0 = true;
        passing0 = 2;
        if (props1 !== true) {
          props1 = props1 || {};
          props1.kind = true;
          props1.prior = true;
        }
      }
    }
  }
  if (!valid0) {
    const err16 = { instancePath, schemaPath: "#/oneOf", keyword: "oneOf", params: { passingSchemas: passing0 }, message: "must match exactly one schema in oneOf" };
    if (vErrors === null) {
      vErrors = [err16];
    } else {
      vErrors.push(err16);
    }
    errors++;
  } else {
    errors = _errs0;
    if (vErrors !== null) {
      if (_errs0) {
        vErrors.length = _errs0;
      } else {
        vErrors = null;
      }
    }
  }
  validate61.errors = vErrors;
  evaluated0.props = props1;
  return errors === 0;
}
validate61.evaluated = { "dynamicProps": true, "dynamicItems": false };
function validate68(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate68.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  const _errs0 = errors;
  let valid0 = false;
  let passing0 = null;
  const _errs1 = errors;
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.kind === void 0) {
      const err0 = { instancePath, schemaPath: "#/oneOf/0/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
      if (vErrors === null) {
        vErrors = [err0];
      } else {
        vErrors.push(err0);
      }
      errors++;
    }
    if (data.sourceId === void 0) {
      const err1 = { instancePath, schemaPath: "#/oneOf/0/required", keyword: "required", params: { missingProperty: "sourceId" }, message: "must have required property 'sourceId'" };
      if (vErrors === null) {
        vErrors = [err1];
      } else {
        vErrors.push(err1);
      }
      errors++;
    }
    for (const key0 in data) {
      if (!(key0 === "kind" || key0 === "sourceId")) {
        const err2 = { instancePath, schemaPath: "#/oneOf/0/additionalProperties", keyword: "additionalProperties", params: { additionalProperty: key0 }, message: "must NOT have additional properties" };
        if (vErrors === null) {
          vErrors = [err2];
        } else {
          vErrors.push(err2);
        }
        errors++;
      }
    }
    if (data.kind !== void 0) {
      let data0 = data.kind;
      if (typeof data0 !== "string") {
        const err3 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/0/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err3];
        } else {
          vErrors.push(err3);
        }
        errors++;
      }
      if ("source" !== data0) {
        const err4 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/0/properties/kind/const", keyword: "const", params: { allowedValue: "source" }, message: "must be equal to constant" };
        if (vErrors === null) {
          vErrors = [err4];
        } else {
          vErrors.push(err4);
        }
        errors++;
      }
    }
    if (data.sourceId !== void 0) {
      if (typeof data.sourceId !== "string") {
        const err5 = { instancePath: instancePath + "/sourceId", schemaPath: "#/oneOf/0/properties/sourceId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err5];
        } else {
          vErrors.push(err5);
        }
        errors++;
      }
    }
  } else {
    const err6 = { instancePath, schemaPath: "#/oneOf/0/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err6];
    } else {
      vErrors.push(err6);
    }
    errors++;
  }
  var _valid0 = _errs1 === errors;
  if (_valid0) {
    valid0 = true;
    passing0 = 0;
    var props0 = true;
  }
  const _errs8 = errors;
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.kind === void 0) {
      const err7 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
      if (vErrors === null) {
        vErrors = [err7];
      } else {
        vErrors.push(err7);
      }
      errors++;
    }
    if (data.componentId === void 0) {
      const err8 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "componentId" }, message: "must have required property 'componentId'" };
      if (vErrors === null) {
        vErrors = [err8];
      } else {
        vErrors.push(err8);
      }
      errors++;
    }
    for (const key1 in data) {
      if (!(key1 === "componentId" || key1 === "kind")) {
        const err9 = { instancePath, schemaPath: "#/oneOf/1/additionalProperties", keyword: "additionalProperties", params: { additionalProperty: key1 }, message: "must NOT have additional properties" };
        if (vErrors === null) {
          vErrors = [err9];
        } else {
          vErrors.push(err9);
        }
        errors++;
      }
    }
    if (data.componentId !== void 0) {
      if (typeof data.componentId !== "string") {
        const err10 = { instancePath: instancePath + "/componentId", schemaPath: "#/oneOf/1/properties/componentId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err10];
        } else {
          vErrors.push(err10);
        }
        errors++;
      }
    }
    if (data.kind !== void 0) {
      let data3 = data.kind;
      if (typeof data3 !== "string") {
        const err11 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/1/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err11];
        } else {
          vErrors.push(err11);
        }
        errors++;
      }
      if ("component" !== data3) {
        const err12 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/1/properties/kind/const", keyword: "const", params: { allowedValue: "component" }, message: "must be equal to constant" };
        if (vErrors === null) {
          vErrors = [err12];
        } else {
          vErrors.push(err12);
        }
        errors++;
      }
    }
  } else {
    const err13 = { instancePath, schemaPath: "#/oneOf/1/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err13];
    } else {
      vErrors.push(err13);
    }
    errors++;
  }
  var _valid0 = _errs8 === errors;
  if (_valid0 && valid0) {
    valid0 = false;
    passing0 = [passing0, 1];
  } else {
    if (_valid0) {
      valid0 = true;
      passing0 = 1;
      if (props0 !== true) {
        props0 = true;
      }
    }
    const _errs15 = errors;
    if (data && typeof data == "object" && !Array.isArray(data)) {
      if (data.kind === void 0) {
        const err14 = { instancePath, schemaPath: "#/oneOf/2/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
        if (vErrors === null) {
          vErrors = [err14];
        } else {
          vErrors.push(err14);
        }
        errors++;
      }
      if (data.moduleId === void 0) {
        const err15 = { instancePath, schemaPath: "#/oneOf/2/required", keyword: "required", params: { missingProperty: "moduleId" }, message: "must have required property 'moduleId'" };
        if (vErrors === null) {
          vErrors = [err15];
        } else {
          vErrors.push(err15);
        }
        errors++;
      }
      for (const key2 in data) {
        if (!(key2 === "kind" || key2 === "moduleId")) {
          const err16 = { instancePath, schemaPath: "#/oneOf/2/additionalProperties", keyword: "additionalProperties", params: { additionalProperty: key2 }, message: "must NOT have additional properties" };
          if (vErrors === null) {
            vErrors = [err16];
          } else {
            vErrors.push(err16);
          }
          errors++;
        }
      }
      if (data.kind !== void 0) {
        let data4 = data.kind;
        if (typeof data4 !== "string") {
          const err17 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/2/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
          if (vErrors === null) {
            vErrors = [err17];
          } else {
            vErrors.push(err17);
          }
          errors++;
        }
        if ("module" !== data4) {
          const err18 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/2/properties/kind/const", keyword: "const", params: { allowedValue: "module" }, message: "must be equal to constant" };
          if (vErrors === null) {
            vErrors = [err18];
          } else {
            vErrors.push(err18);
          }
          errors++;
        }
      }
      if (data.moduleId !== void 0) {
        if (typeof data.moduleId !== "string") {
          const err19 = { instancePath: instancePath + "/moduleId", schemaPath: "#/oneOf/2/properties/moduleId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
          if (vErrors === null) {
            vErrors = [err19];
          } else {
            vErrors.push(err19);
          }
          errors++;
        }
      }
    } else {
      const err20 = { instancePath, schemaPath: "#/oneOf/2/type", keyword: "type", params: { type: "object" }, message: "must be object" };
      if (vErrors === null) {
        vErrors = [err20];
      } else {
        vErrors.push(err20);
      }
      errors++;
    }
    var _valid0 = _errs15 === errors;
    if (_valid0 && valid0) {
      valid0 = false;
      passing0 = [passing0, 2];
    } else {
      if (_valid0) {
        valid0 = true;
        passing0 = 2;
        if (props0 !== true) {
          props0 = true;
        }
      }
      const _errs22 = errors;
      if (data && typeof data == "object" && !Array.isArray(data)) {
        if (data.kind === void 0) {
          const err21 = { instancePath, schemaPath: "#/oneOf/3/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
          if (vErrors === null) {
            vErrors = [err21];
          } else {
            vErrors.push(err21);
          }
          errors++;
        }
        if (data.ownerModuleId === void 0) {
          const err22 = { instancePath, schemaPath: "#/oneOf/3/required", keyword: "required", params: { missingProperty: "ownerModuleId" }, message: "must have required property 'ownerModuleId'" };
          if (vErrors === null) {
            vErrors = [err22];
          } else {
            vErrors.push(err22);
          }
          errors++;
        }
        if (data.namespaceId === void 0) {
          const err23 = { instancePath, schemaPath: "#/oneOf/3/required", keyword: "required", params: { missingProperty: "namespaceId" }, message: "must have required property 'namespaceId'" };
          if (vErrors === null) {
            vErrors = [err23];
          } else {
            vErrors.push(err23);
          }
          errors++;
        }
        if (data.declarationId === void 0) {
          const err24 = { instancePath, schemaPath: "#/oneOf/3/required", keyword: "required", params: { missingProperty: "declarationId" }, message: "must have required property 'declarationId'" };
          if (vErrors === null) {
            vErrors = [err24];
          } else {
            vErrors.push(err24);
          }
          errors++;
        }
        for (const key3 in data) {
          if (!(key3 === "declarationId" || key3 === "kind" || key3 === "namespaceId" || key3 === "ownerModuleId")) {
            const err25 = { instancePath, schemaPath: "#/oneOf/3/additionalProperties", keyword: "additionalProperties", params: { additionalProperty: key3 }, message: "must NOT have additional properties" };
            if (vErrors === null) {
              vErrors = [err25];
            } else {
              vErrors.push(err25);
            }
            errors++;
          }
        }
        if (data.declarationId !== void 0) {
          if (typeof data.declarationId !== "string") {
            const err26 = { instancePath: instancePath + "/declarationId", schemaPath: "#/oneOf/3/properties/declarationId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
            if (vErrors === null) {
              vErrors = [err26];
            } else {
              vErrors.push(err26);
            }
            errors++;
          }
        }
        if (data.kind !== void 0) {
          let data7 = data.kind;
          if (typeof data7 !== "string") {
            const err27 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/3/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
            if (vErrors === null) {
              vErrors = [err27];
            } else {
              vErrors.push(err27);
            }
            errors++;
          }
          if ("declaration" !== data7) {
            const err28 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/3/properties/kind/const", keyword: "const", params: { allowedValue: "declaration" }, message: "must be equal to constant" };
            if (vErrors === null) {
              vErrors = [err28];
            } else {
              vErrors.push(err28);
            }
            errors++;
          }
        }
        if (data.namespaceId !== void 0) {
          if (typeof data.namespaceId !== "string") {
            const err29 = { instancePath: instancePath + "/namespaceId", schemaPath: "#/oneOf/3/properties/namespaceId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
            if (vErrors === null) {
              vErrors = [err29];
            } else {
              vErrors.push(err29);
            }
            errors++;
          }
        }
        if (data.ownerModuleId !== void 0) {
          if (typeof data.ownerModuleId !== "string") {
            const err30 = { instancePath: instancePath + "/ownerModuleId", schemaPath: "#/oneOf/3/properties/ownerModuleId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
            if (vErrors === null) {
              vErrors = [err30];
            } else {
              vErrors.push(err30);
            }
            errors++;
          }
        }
      } else {
        const err31 = { instancePath, schemaPath: "#/oneOf/3/type", keyword: "type", params: { type: "object" }, message: "must be object" };
        if (vErrors === null) {
          vErrors = [err31];
        } else {
          vErrors.push(err31);
        }
        errors++;
      }
      var _valid0 = _errs22 === errors;
      if (_valid0 && valid0) {
        valid0 = false;
        passing0 = [passing0, 3];
      } else {
        if (_valid0) {
          valid0 = true;
          passing0 = 3;
          if (props0 !== true) {
            props0 = true;
          }
        }
        const _errs33 = errors;
        if (data && typeof data == "object" && !Array.isArray(data)) {
          if (data.kind === void 0) {
            const err32 = { instancePath, schemaPath: "#/oneOf/4/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
            if (vErrors === null) {
              vErrors = [err32];
            } else {
              vErrors.push(err32);
            }
            errors++;
          }
          if (data.importerModuleId === void 0) {
            const err33 = { instancePath, schemaPath: "#/oneOf/4/required", keyword: "required", params: { missingProperty: "importerModuleId" }, message: "must have required property 'importerModuleId'" };
            if (vErrors === null) {
              vErrors = [err33];
            } else {
              vErrors.push(err33);
            }
            errors++;
          }
          if (data.upstreamModuleId === void 0) {
            const err34 = { instancePath, schemaPath: "#/oneOf/4/required", keyword: "required", params: { missingProperty: "upstreamModuleId" }, message: "must have required property 'upstreamModuleId'" };
            if (vErrors === null) {
              vErrors = [err34];
            } else {
              vErrors.push(err34);
            }
            errors++;
          }
          if (data.upstreamReleaseId === void 0) {
            const err35 = { instancePath, schemaPath: "#/oneOf/4/required", keyword: "required", params: { missingProperty: "upstreamReleaseId" }, message: "must have required property 'upstreamReleaseId'" };
            if (vErrors === null) {
              vErrors = [err35];
            } else {
              vErrors.push(err35);
            }
            errors++;
          }
          if (data.exportId === void 0) {
            const err36 = { instancePath, schemaPath: "#/oneOf/4/required", keyword: "required", params: { missingProperty: "exportId" }, message: "must have required property 'exportId'" };
            if (vErrors === null) {
              vErrors = [err36];
            } else {
              vErrors.push(err36);
            }
            errors++;
          }
          if (data.sourceNamespaceId === void 0) {
            const err37 = { instancePath, schemaPath: "#/oneOf/4/required", keyword: "required", params: { missingProperty: "sourceNamespaceId" }, message: "must have required property 'sourceNamespaceId'" };
            if (vErrors === null) {
              vErrors = [err37];
            } else {
              vErrors.push(err37);
            }
            errors++;
          }
          if (data.localNamespaceId === void 0) {
            const err38 = { instancePath, schemaPath: "#/oneOf/4/required", keyword: "required", params: { missingProperty: "localNamespaceId" }, message: "must have required property 'localNamespaceId'" };
            if (vErrors === null) {
              vErrors = [err38];
            } else {
              vErrors.push(err38);
            }
            errors++;
          }
          for (const key4 in data) {
            if (!(key4 === "exportId" || key4 === "importerModuleId" || key4 === "kind" || key4 === "localNamespaceId" || key4 === "sourceNamespaceId" || key4 === "upstreamModuleId" || key4 === "upstreamReleaseId")) {
              const err39 = { instancePath, schemaPath: "#/oneOf/4/additionalProperties", keyword: "additionalProperties", params: { additionalProperty: key4 }, message: "must NOT have additional properties" };
              if (vErrors === null) {
                vErrors = [err39];
              } else {
                vErrors.push(err39);
              }
              errors++;
            }
          }
          if (data.exportId !== void 0) {
            if (typeof data.exportId !== "string") {
              const err40 = { instancePath: instancePath + "/exportId", schemaPath: "#/oneOf/4/properties/exportId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
              if (vErrors === null) {
                vErrors = [err40];
              } else {
                vErrors.push(err40);
              }
              errors++;
            }
          }
          if (data.importerModuleId !== void 0) {
            if (typeof data.importerModuleId !== "string") {
              const err41 = { instancePath: instancePath + "/importerModuleId", schemaPath: "#/oneOf/4/properties/importerModuleId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
              if (vErrors === null) {
                vErrors = [err41];
              } else {
                vErrors.push(err41);
              }
              errors++;
            }
          }
          if (data.kind !== void 0) {
            let data12 = data.kind;
            if (typeof data12 !== "string") {
              const err42 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/4/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
              if (vErrors === null) {
                vErrors = [err42];
              } else {
                vErrors.push(err42);
              }
              errors++;
            }
            if ("import" !== data12) {
              const err43 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/4/properties/kind/const", keyword: "const", params: { allowedValue: "import" }, message: "must be equal to constant" };
              if (vErrors === null) {
                vErrors = [err43];
              } else {
                vErrors.push(err43);
              }
              errors++;
            }
          }
          if (data.localNamespaceId !== void 0) {
            if (typeof data.localNamespaceId !== "string") {
              const err44 = { instancePath: instancePath + "/localNamespaceId", schemaPath: "#/oneOf/4/properties/localNamespaceId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
              if (vErrors === null) {
                vErrors = [err44];
              } else {
                vErrors.push(err44);
              }
              errors++;
            }
          }
          if (data.sourceNamespaceId !== void 0) {
            if (typeof data.sourceNamespaceId !== "string") {
              const err45 = { instancePath: instancePath + "/sourceNamespaceId", schemaPath: "#/oneOf/4/properties/sourceNamespaceId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
              if (vErrors === null) {
                vErrors = [err45];
              } else {
                vErrors.push(err45);
              }
              errors++;
            }
          }
          if (data.upstreamModuleId !== void 0) {
            if (typeof data.upstreamModuleId !== "string") {
              const err46 = { instancePath: instancePath + "/upstreamModuleId", schemaPath: "#/oneOf/4/properties/upstreamModuleId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
              if (vErrors === null) {
                vErrors = [err46];
              } else {
                vErrors.push(err46);
              }
              errors++;
            }
          }
          if (data.upstreamReleaseId !== void 0) {
            if (typeof data.upstreamReleaseId !== "string") {
              const err47 = { instancePath: instancePath + "/upstreamReleaseId", schemaPath: "#/oneOf/4/properties/upstreamReleaseId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
              if (vErrors === null) {
                vErrors = [err47];
              } else {
                vErrors.push(err47);
              }
              errors++;
            }
          }
        } else {
          const err48 = { instancePath, schemaPath: "#/oneOf/4/type", keyword: "type", params: { type: "object" }, message: "must be object" };
          if (vErrors === null) {
            vErrors = [err48];
          } else {
            vErrors.push(err48);
          }
          errors++;
        }
        var _valid0 = _errs33 === errors;
        if (_valid0 && valid0) {
          valid0 = false;
          passing0 = [passing0, 4];
        } else {
          if (_valid0) {
            valid0 = true;
            passing0 = 4;
            if (props0 !== true) {
              props0 = true;
            }
          }
          const _errs50 = errors;
          if (data && typeof data == "object" && !Array.isArray(data)) {
            if (data.kind === void 0) {
              const err49 = { instancePath, schemaPath: "#/oneOf/5/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
              if (vErrors === null) {
                vErrors = [err49];
              } else {
                vErrors.push(err49);
              }
              errors++;
            }
            if (data.ownerModuleId === void 0) {
              const err50 = { instancePath, schemaPath: "#/oneOf/5/required", keyword: "required", params: { missingProperty: "ownerModuleId" }, message: "must have required property 'ownerModuleId'" };
              if (vErrors === null) {
                vErrors = [err50];
              } else {
                vErrors.push(err50);
              }
              errors++;
            }
            if (data.extensionNamespaceId === void 0) {
              const err51 = { instancePath, schemaPath: "#/oneOf/5/required", keyword: "required", params: { missingProperty: "extensionNamespaceId" }, message: "must have required property 'extensionNamespaceId'" };
              if (vErrors === null) {
                vErrors = [err51];
              } else {
                vErrors.push(err51);
              }
              errors++;
            }
            if (data.targetNamespaceId === void 0) {
              const err52 = { instancePath, schemaPath: "#/oneOf/5/required", keyword: "required", params: { missingProperty: "targetNamespaceId" }, message: "must have required property 'targetNamespaceId'" };
              if (vErrors === null) {
                vErrors = [err52];
              } else {
                vErrors.push(err52);
              }
              errors++;
            }
            if (data.targetDeclarationId === void 0) {
              const err53 = { instancePath, schemaPath: "#/oneOf/5/required", keyword: "required", params: { missingProperty: "targetDeclarationId" }, message: "must have required property 'targetDeclarationId'" };
              if (vErrors === null) {
                vErrors = [err53];
              } else {
                vErrors.push(err53);
              }
              errors++;
            }
            for (const key5 in data) {
              if (!(key5 === "extensionNamespaceId" || key5 === "kind" || key5 === "ownerModuleId" || key5 === "targetDeclarationId" || key5 === "targetNamespaceId")) {
                const err54 = { instancePath, schemaPath: "#/oneOf/5/additionalProperties", keyword: "additionalProperties", params: { additionalProperty: key5 }, message: "must NOT have additional properties" };
                if (vErrors === null) {
                  vErrors = [err54];
                } else {
                  vErrors.push(err54);
                }
                errors++;
              }
            }
            if (data.extensionNamespaceId !== void 0) {
              if (typeof data.extensionNamespaceId !== "string") {
                const err55 = { instancePath: instancePath + "/extensionNamespaceId", schemaPath: "#/oneOf/5/properties/extensionNamespaceId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                if (vErrors === null) {
                  vErrors = [err55];
                } else {
                  vErrors.push(err55);
                }
                errors++;
              }
            }
            if (data.kind !== void 0) {
              let data18 = data.kind;
              if (typeof data18 !== "string") {
                const err56 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/5/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                if (vErrors === null) {
                  vErrors = [err56];
                } else {
                  vErrors.push(err56);
                }
                errors++;
              }
              if ("extension" !== data18) {
                const err57 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/5/properties/kind/const", keyword: "const", params: { allowedValue: "extension" }, message: "must be equal to constant" };
                if (vErrors === null) {
                  vErrors = [err57];
                } else {
                  vErrors.push(err57);
                }
                errors++;
              }
            }
            if (data.ownerModuleId !== void 0) {
              if (typeof data.ownerModuleId !== "string") {
                const err58 = { instancePath: instancePath + "/ownerModuleId", schemaPath: "#/oneOf/5/properties/ownerModuleId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                if (vErrors === null) {
                  vErrors = [err58];
                } else {
                  vErrors.push(err58);
                }
                errors++;
              }
            }
            if (data.targetDeclarationId !== void 0) {
              if (typeof data.targetDeclarationId !== "string") {
                const err59 = { instancePath: instancePath + "/targetDeclarationId", schemaPath: "#/oneOf/5/properties/targetDeclarationId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                if (vErrors === null) {
                  vErrors = [err59];
                } else {
                  vErrors.push(err59);
                }
                errors++;
              }
            }
            if (data.targetNamespaceId !== void 0) {
              if (typeof data.targetNamespaceId !== "string") {
                const err60 = { instancePath: instancePath + "/targetNamespaceId", schemaPath: "#/oneOf/5/properties/targetNamespaceId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                if (vErrors === null) {
                  vErrors = [err60];
                } else {
                  vErrors.push(err60);
                }
                errors++;
              }
            }
          } else {
            const err61 = { instancePath, schemaPath: "#/oneOf/5/type", keyword: "type", params: { type: "object" }, message: "must be object" };
            if (vErrors === null) {
              vErrors = [err61];
            } else {
              vErrors.push(err61);
            }
            errors++;
          }
          var _valid0 = _errs50 === errors;
          if (_valid0 && valid0) {
            valid0 = false;
            passing0 = [passing0, 5];
          } else {
            if (_valid0) {
              valid0 = true;
              passing0 = 5;
              if (props0 !== true) {
                props0 = true;
              }
            }
            const _errs63 = errors;
            if (data && typeof data == "object" && !Array.isArray(data)) {
              if (data.kind === void 0) {
                const err62 = { instancePath, schemaPath: "#/oneOf/6/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
                if (vErrors === null) {
                  vErrors = [err62];
                } else {
                  vErrors.push(err62);
                }
                errors++;
              }
              if (data.ownerModuleId === void 0) {
                const err63 = { instancePath, schemaPath: "#/oneOf/6/required", keyword: "required", params: { missingProperty: "ownerModuleId" }, message: "must have required property 'ownerModuleId'" };
                if (vErrors === null) {
                  vErrors = [err63];
                } else {
                  vErrors.push(err63);
                }
                errors++;
              }
              if (data.namespaceId === void 0) {
                const err64 = { instancePath, schemaPath: "#/oneOf/6/required", keyword: "required", params: { missingProperty: "namespaceId" }, message: "must have required property 'namespaceId'" };
                if (vErrors === null) {
                  vErrors = [err64];
                } else {
                  vErrors.push(err64);
                }
                errors++;
              }
              if (data.declarationId === void 0) {
                const err65 = { instancePath, schemaPath: "#/oneOf/6/required", keyword: "required", params: { missingProperty: "declarationId" }, message: "must have required property 'declarationId'" };
                if (vErrors === null) {
                  vErrors = [err65];
                } else {
                  vErrors.push(err65);
                }
                errors++;
              }
              for (const key6 in data) {
                if (!(key6 === "declarationId" || key6 === "kind" || key6 === "namespaceId" || key6 === "ownerModuleId")) {
                  const err66 = { instancePath, schemaPath: "#/oneOf/6/additionalProperties", keyword: "additionalProperties", params: { additionalProperty: key6 }, message: "must NOT have additional properties" };
                  if (vErrors === null) {
                    vErrors = [err66];
                  } else {
                    vErrors.push(err66);
                  }
                  errors++;
                }
              }
              if (data.declarationId !== void 0) {
                if (typeof data.declarationId !== "string") {
                  const err67 = { instancePath: instancePath + "/declarationId", schemaPath: "#/oneOf/6/properties/declarationId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                  if (vErrors === null) {
                    vErrors = [err67];
                  } else {
                    vErrors.push(err67);
                  }
                  errors++;
                }
              }
              if (data.kind !== void 0) {
                let data23 = data.kind;
                if (typeof data23 !== "string") {
                  const err68 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/6/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                  if (vErrors === null) {
                    vErrors = [err68];
                  } else {
                    vErrors.push(err68);
                  }
                  errors++;
                }
                if ("policy" !== data23) {
                  const err69 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/6/properties/kind/const", keyword: "const", params: { allowedValue: "policy" }, message: "must be equal to constant" };
                  if (vErrors === null) {
                    vErrors = [err69];
                  } else {
                    vErrors.push(err69);
                  }
                  errors++;
                }
              }
              if (data.namespaceId !== void 0) {
                if (typeof data.namespaceId !== "string") {
                  const err70 = { instancePath: instancePath + "/namespaceId", schemaPath: "#/oneOf/6/properties/namespaceId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                  if (vErrors === null) {
                    vErrors = [err70];
                  } else {
                    vErrors.push(err70);
                  }
                  errors++;
                }
              }
              if (data.ownerModuleId !== void 0) {
                if (typeof data.ownerModuleId !== "string") {
                  const err71 = { instancePath: instancePath + "/ownerModuleId", schemaPath: "#/oneOf/6/properties/ownerModuleId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                  if (vErrors === null) {
                    vErrors = [err71];
                  } else {
                    vErrors.push(err71);
                  }
                  errors++;
                }
              }
            } else {
              const err72 = { instancePath, schemaPath: "#/oneOf/6/type", keyword: "type", params: { type: "object" }, message: "must be object" };
              if (vErrors === null) {
                vErrors = [err72];
              } else {
                vErrors.push(err72);
              }
              errors++;
            }
            var _valid0 = _errs63 === errors;
            if (_valid0 && valid0) {
              valid0 = false;
              passing0 = [passing0, 6];
            } else {
              if (_valid0) {
                valid0 = true;
                passing0 = 6;
                if (props0 !== true) {
                  props0 = true;
                }
              }
              const _errs74 = errors;
              if (data && typeof data == "object" && !Array.isArray(data)) {
                if (data.kind === void 0) {
                  const err73 = { instancePath, schemaPath: "#/oneOf/7/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
                  if (vErrors === null) {
                    vErrors = [err73];
                  } else {
                    vErrors.push(err73);
                  }
                  errors++;
                }
                for (const key7 in data) {
                  if (!(key7 === "kind")) {
                    const err74 = { instancePath, schemaPath: "#/oneOf/7/additionalProperties", keyword: "additionalProperties", params: { additionalProperty: key7 }, message: "must NOT have additional properties" };
                    if (vErrors === null) {
                      vErrors = [err74];
                    } else {
                      vErrors.push(err74);
                    }
                    errors++;
                  }
                }
                if (data.kind !== void 0) {
                  let data26 = data.kind;
                  if (typeof data26 !== "string") {
                    const err75 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/7/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                    if (vErrors === null) {
                      vErrors = [err75];
                    } else {
                      vErrors.push(err75);
                    }
                    errors++;
                  }
                  if ("responsibility-closure" !== data26) {
                    const err76 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/7/properties/kind/const", keyword: "const", params: { allowedValue: "responsibility-closure" }, message: "must be equal to constant" };
                    if (vErrors === null) {
                      vErrors = [err76];
                    } else {
                      vErrors.push(err76);
                    }
                    errors++;
                  }
                }
              } else {
                const err77 = { instancePath, schemaPath: "#/oneOf/7/type", keyword: "type", params: { type: "object" }, message: "must be object" };
                if (vErrors === null) {
                  vErrors = [err77];
                } else {
                  vErrors.push(err77);
                }
                errors++;
              }
              var _valid0 = _errs74 === errors;
              if (_valid0 && valid0) {
                valid0 = false;
                passing0 = [passing0, 7];
              } else {
                if (_valid0) {
                  valid0 = true;
                  passing0 = 7;
                  if (props0 !== true) {
                    props0 = true;
                  }
                }
                const _errs79 = errors;
                if (data && typeof data == "object" && !Array.isArray(data)) {
                  if (data.kind === void 0) {
                    const err78 = { instancePath, schemaPath: "#/oneOf/8/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
                    if (vErrors === null) {
                      vErrors = [err78];
                    } else {
                      vErrors.push(err78);
                    }
                    errors++;
                  }
                  for (const key8 in data) {
                    if (!(key8 === "kind")) {
                      const err79 = { instancePath, schemaPath: "#/oneOf/8/additionalProperties", keyword: "additionalProperties", params: { additionalProperty: key8 }, message: "must NOT have additional properties" };
                      if (vErrors === null) {
                        vErrors = [err79];
                      } else {
                        vErrors.push(err79);
                      }
                      errors++;
                    }
                  }
                  if (data.kind !== void 0) {
                    let data27 = data.kind;
                    if (typeof data27 !== "string") {
                      const err80 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/8/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                      if (vErrors === null) {
                        vErrors = [err80];
                      } else {
                        vErrors.push(err80);
                      }
                      errors++;
                    }
                    if ("organizational-translation-closure" !== data27) {
                      const err81 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/8/properties/kind/const", keyword: "const", params: { allowedValue: "organizational-translation-closure" }, message: "must be equal to constant" };
                      if (vErrors === null) {
                        vErrors = [err81];
                      } else {
                        vErrors.push(err81);
                      }
                      errors++;
                    }
                  }
                } else {
                  const err82 = { instancePath, schemaPath: "#/oneOf/8/type", keyword: "type", params: { type: "object" }, message: "must be object" };
                  if (vErrors === null) {
                    vErrors = [err82];
                  } else {
                    vErrors.push(err82);
                  }
                  errors++;
                }
                var _valid0 = _errs79 === errors;
                if (_valid0 && valid0) {
                  valid0 = false;
                  passing0 = [passing0, 8];
                } else {
                  if (_valid0) {
                    valid0 = true;
                    passing0 = 8;
                    if (props0 !== true) {
                      props0 = true;
                    }
                  }
                  const _errs84 = errors;
                  if (data && typeof data == "object" && !Array.isArray(data)) {
                    if (data.kind === void 0) {
                      const err83 = { instancePath, schemaPath: "#/oneOf/9/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
                      if (vErrors === null) {
                        vErrors = [err83];
                      } else {
                        vErrors.push(err83);
                      }
                      errors++;
                    }
                    for (const key9 in data) {
                      if (!(key9 === "kind")) {
                        const err84 = { instancePath, schemaPath: "#/oneOf/9/additionalProperties", keyword: "additionalProperties", params: { additionalProperty: key9 }, message: "must NOT have additional properties" };
                        if (vErrors === null) {
                          vErrors = [err84];
                        } else {
                          vErrors.push(err84);
                        }
                        errors++;
                      }
                    }
                    if (data.kind !== void 0) {
                      let data28 = data.kind;
                      if (typeof data28 !== "string") {
                        const err85 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/9/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                        if (vErrors === null) {
                          vErrors = [err85];
                        } else {
                          vErrors.push(err85);
                        }
                        errors++;
                      }
                      if ("experience-composition" !== data28) {
                        const err86 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/9/properties/kind/const", keyword: "const", params: { allowedValue: "experience-composition" }, message: "must be equal to constant" };
                        if (vErrors === null) {
                          vErrors = [err86];
                        } else {
                          vErrors.push(err86);
                        }
                        errors++;
                      }
                    }
                  } else {
                    const err87 = { instancePath, schemaPath: "#/oneOf/9/type", keyword: "type", params: { type: "object" }, message: "must be object" };
                    if (vErrors === null) {
                      vErrors = [err87];
                    } else {
                      vErrors.push(err87);
                    }
                    errors++;
                  }
                  var _valid0 = _errs84 === errors;
                  if (_valid0 && valid0) {
                    valid0 = false;
                    passing0 = [passing0, 9];
                  } else {
                    if (_valid0) {
                      valid0 = true;
                      passing0 = 9;
                      if (props0 !== true) {
                        props0 = true;
                      }
                    }
                    const _errs89 = errors;
                    if (data && typeof data == "object" && !Array.isArray(data)) {
                      if (data.kind === void 0) {
                        const err88 = { instancePath, schemaPath: "#/oneOf/10/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
                        if (vErrors === null) {
                          vErrors = [err88];
                        } else {
                          vErrors.push(err88);
                        }
                        errors++;
                      }
                      if (data.ownerModuleId === void 0) {
                        const err89 = { instancePath, schemaPath: "#/oneOf/10/required", keyword: "required", params: { missingProperty: "ownerModuleId" }, message: "must have required property 'ownerModuleId'" };
                        if (vErrors === null) {
                          vErrors = [err89];
                        } else {
                          vErrors.push(err89);
                        }
                        errors++;
                      }
                      if (data.resourceId === void 0) {
                        const err90 = { instancePath, schemaPath: "#/oneOf/10/required", keyword: "required", params: { missingProperty: "resourceId" }, message: "must have required property 'resourceId'" };
                        if (vErrors === null) {
                          vErrors = [err90];
                        } else {
                          vErrors.push(err90);
                        }
                        errors++;
                      }
                      for (const key10 in data) {
                        if (!(key10 === "kind" || key10 === "ownerModuleId" || key10 === "resourceId")) {
                          const err91 = { instancePath, schemaPath: "#/oneOf/10/additionalProperties", keyword: "additionalProperties", params: { additionalProperty: key10 }, message: "must NOT have additional properties" };
                          if (vErrors === null) {
                            vErrors = [err91];
                          } else {
                            vErrors.push(err91);
                          }
                          errors++;
                        }
                      }
                      if (data.kind !== void 0) {
                        let data29 = data.kind;
                        if (typeof data29 !== "string") {
                          const err92 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/10/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                          if (vErrors === null) {
                            vErrors = [err92];
                          } else {
                            vErrors.push(err92);
                          }
                          errors++;
                        }
                        if ("resource" !== data29) {
                          const err93 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/10/properties/kind/const", keyword: "const", params: { allowedValue: "resource" }, message: "must be equal to constant" };
                          if (vErrors === null) {
                            vErrors = [err93];
                          } else {
                            vErrors.push(err93);
                          }
                          errors++;
                        }
                      }
                      if (data.ownerModuleId !== void 0) {
                        if (typeof data.ownerModuleId !== "string") {
                          const err94 = { instancePath: instancePath + "/ownerModuleId", schemaPath: "#/oneOf/10/properties/ownerModuleId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                          if (vErrors === null) {
                            vErrors = [err94];
                          } else {
                            vErrors.push(err94);
                          }
                          errors++;
                        }
                      }
                      if (data.resourceId !== void 0) {
                        if (typeof data.resourceId !== "string") {
                          const err95 = { instancePath: instancePath + "/resourceId", schemaPath: "#/oneOf/10/properties/resourceId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                          if (vErrors === null) {
                            vErrors = [err95];
                          } else {
                            vErrors.push(err95);
                          }
                          errors++;
                        }
                      }
                    } else {
                      const err96 = { instancePath, schemaPath: "#/oneOf/10/type", keyword: "type", params: { type: "object" }, message: "must be object" };
                      if (vErrors === null) {
                        vErrors = [err96];
                      } else {
                        vErrors.push(err96);
                      }
                      errors++;
                    }
                    var _valid0 = _errs89 === errors;
                    if (_valid0 && valid0) {
                      valid0 = false;
                      passing0 = [passing0, 10];
                    } else {
                      if (_valid0) {
                        valid0 = true;
                        passing0 = 10;
                        if (props0 !== true) {
                          props0 = true;
                        }
                      }
                      const _errs98 = errors;
                      if (data && typeof data == "object" && !Array.isArray(data)) {
                        if (data.kind === void 0) {
                          const err97 = { instancePath, schemaPath: "#/oneOf/11/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
                          if (vErrors === null) {
                            vErrors = [err97];
                          } else {
                            vErrors.push(err97);
                          }
                          errors++;
                        }
                        for (const key11 in data) {
                          if (!(key11 === "kind")) {
                            const err98 = { instancePath, schemaPath: "#/oneOf/11/additionalProperties", keyword: "additionalProperties", params: { additionalProperty: key11 }, message: "must NOT have additional properties" };
                            if (vErrors === null) {
                              vErrors = [err98];
                            } else {
                              vErrors.push(err98);
                            }
                            errors++;
                          }
                        }
                        if (data.kind !== void 0) {
                          let data32 = data.kind;
                          if (typeof data32 !== "string") {
                            const err99 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/11/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                            if (vErrors === null) {
                              vErrors = [err99];
                            } else {
                              vErrors.push(err99);
                            }
                            errors++;
                          }
                          if ("resolution-lock" !== data32) {
                            const err100 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/11/properties/kind/const", keyword: "const", params: { allowedValue: "resolution-lock" }, message: "must be equal to constant" };
                            if (vErrors === null) {
                              vErrors = [err100];
                            } else {
                              vErrors.push(err100);
                            }
                            errors++;
                          }
                        }
                      } else {
                        const err101 = { instancePath, schemaPath: "#/oneOf/11/type", keyword: "type", params: { type: "object" }, message: "must be object" };
                        if (vErrors === null) {
                          vErrors = [err101];
                        } else {
                          vErrors.push(err101);
                        }
                        errors++;
                      }
                      var _valid0 = _errs98 === errors;
                      if (_valid0 && valid0) {
                        valid0 = false;
                        passing0 = [passing0, 11];
                      } else {
                        if (_valid0) {
                          valid0 = true;
                          passing0 = 11;
                          if (props0 !== true) {
                            props0 = true;
                          }
                        }
                        const _errs103 = errors;
                        if (data && typeof data == "object" && !Array.isArray(data)) {
                          if (data.kind === void 0) {
                            const err102 = { instancePath, schemaPath: "#/oneOf/12/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
                            if (vErrors === null) {
                              vErrors = [err102];
                            } else {
                              vErrors.push(err102);
                            }
                            errors++;
                          }
                          for (const key12 in data) {
                            if (!(key12 === "kind")) {
                              const err103 = { instancePath, schemaPath: "#/oneOf/12/additionalProperties", keyword: "additionalProperties", params: { additionalProperty: key12 }, message: "must NOT have additional properties" };
                              if (vErrors === null) {
                                vErrors = [err103];
                              } else {
                                vErrors.push(err103);
                              }
                              errors++;
                            }
                          }
                          if (data.kind !== void 0) {
                            let data33 = data.kind;
                            if (typeof data33 !== "string") {
                              const err104 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/12/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                              if (vErrors === null) {
                                vErrors = [err104];
                              } else {
                                vErrors.push(err104);
                              }
                              errors++;
                            }
                            if ("universe-lock" !== data33) {
                              const err105 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/12/properties/kind/const", keyword: "const", params: { allowedValue: "universe-lock" }, message: "must be equal to constant" };
                              if (vErrors === null) {
                                vErrors = [err105];
                              } else {
                                vErrors.push(err105);
                              }
                              errors++;
                            }
                          }
                        } else {
                          const err106 = { instancePath, schemaPath: "#/oneOf/12/type", keyword: "type", params: { type: "object" }, message: "must be object" };
                          if (vErrors === null) {
                            vErrors = [err106];
                          } else {
                            vErrors.push(err106);
                          }
                          errors++;
                        }
                        var _valid0 = _errs103 === errors;
                        if (_valid0 && valid0) {
                          valid0 = false;
                          passing0 = [passing0, 12];
                        } else {
                          if (_valid0) {
                            valid0 = true;
                            passing0 = 12;
                            if (props0 !== true) {
                              props0 = true;
                            }
                          }
                          const _errs108 = errors;
                          if (data && typeof data == "object" && !Array.isArray(data)) {
                            if (data.kind === void 0) {
                              const err107 = { instancePath, schemaPath: "#/oneOf/13/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
                              if (vErrors === null) {
                                vErrors = [err107];
                              } else {
                                vErrors.push(err107);
                              }
                              errors++;
                            }
                            if (data.workspaceId === void 0) {
                              const err108 = { instancePath, schemaPath: "#/oneOf/13/required", keyword: "required", params: { missingProperty: "workspaceId" }, message: "must have required property 'workspaceId'" };
                              if (vErrors === null) {
                                vErrors = [err108];
                              } else {
                                vErrors.push(err108);
                              }
                              errors++;
                            }
                            for (const key13 in data) {
                              if (!(key13 === "kind" || key13 === "workspaceId")) {
                                const err109 = { instancePath, schemaPath: "#/oneOf/13/additionalProperties", keyword: "additionalProperties", params: { additionalProperty: key13 }, message: "must NOT have additional properties" };
                                if (vErrors === null) {
                                  vErrors = [err109];
                                } else {
                                  vErrors.push(err109);
                                }
                                errors++;
                              }
                            }
                            if (data.kind !== void 0) {
                              let data34 = data.kind;
                              if (typeof data34 !== "string") {
                                const err110 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/13/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                                if (vErrors === null) {
                                  vErrors = [err110];
                                } else {
                                  vErrors.push(err110);
                                }
                                errors++;
                              }
                              if ("workspace" !== data34) {
                                const err111 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/13/properties/kind/const", keyword: "const", params: { allowedValue: "workspace" }, message: "must be equal to constant" };
                                if (vErrors === null) {
                                  vErrors = [err111];
                                } else {
                                  vErrors.push(err111);
                                }
                                errors++;
                              }
                            }
                            if (data.workspaceId !== void 0) {
                              if (typeof data.workspaceId !== "string") {
                                const err112 = { instancePath: instancePath + "/workspaceId", schemaPath: "#/oneOf/13/properties/workspaceId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                                if (vErrors === null) {
                                  vErrors = [err112];
                                } else {
                                  vErrors.push(err112);
                                }
                                errors++;
                              }
                            }
                          } else {
                            const err113 = { instancePath, schemaPath: "#/oneOf/13/type", keyword: "type", params: { type: "object" }, message: "must be object" };
                            if (vErrors === null) {
                              vErrors = [err113];
                            } else {
                              vErrors.push(err113);
                            }
                            errors++;
                          }
                          var _valid0 = _errs108 === errors;
                          if (_valid0 && valid0) {
                            valid0 = false;
                            passing0 = [passing0, 13];
                          } else {
                            if (_valid0) {
                              valid0 = true;
                              passing0 = 13;
                              if (props0 !== true) {
                                props0 = true;
                              }
                            }
                          }
                        }
                      }
                    }
                  }
                }
              }
            }
          }
        }
      }
    }
  }
  if (!valid0) {
    const err114 = { instancePath, schemaPath: "#/oneOf", keyword: "oneOf", params: { passingSchemas: passing0 }, message: "must match exactly one schema in oneOf" };
    if (vErrors === null) {
      vErrors = [err114];
    } else {
      vErrors.push(err114);
    }
    errors++;
  } else {
    errors = _errs0;
    if (vErrors !== null) {
      if (_errs0) {
        vErrors.length = _errs0;
      } else {
        vErrors = null;
      }
    }
  }
  validate68.errors = vErrors;
  evaluated0.props = props0;
  return errors === 0;
}
validate68.evaluated = { "dynamicProps": true, "dynamicItems": false };
function validate60(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate60.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.rootDigest === void 0) {
      const err0 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "rootDigest" }, message: "must have required property 'rootDigest'" };
      if (vErrors === null) {
        vErrors = [err0];
      } else {
        vErrors.push(err0);
      }
      errors++;
    }
    if (data.subject === void 0) {
      const err1 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "subject" }, message: "must have required property 'subject'" };
      if (vErrors === null) {
        vErrors = [err1];
      } else {
        vErrors.push(err1);
      }
      errors++;
    }
    if (data.change === void 0) {
      const err2 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "change" }, message: "must have required property 'change'" };
      if (vErrors === null) {
        vErrors = [err2];
      } else {
        vErrors.push(err2);
      }
      errors++;
    }
    if (data.change !== void 0) {
      if (!validate61(data.change, { instancePath: instancePath + "/change", parentData: data, parentDataProperty: "change", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate61.errors : vErrors.concat(validate61.errors);
        errors = vErrors.length;
      }
    }
    if (data.rootDigest !== void 0) {
      if (typeof data.rootDigest !== "string") {
        const err3 = { instancePath: instancePath + "/rootDigest", schemaPath: "#/properties/rootDigest/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err3];
        } else {
          vErrors.push(err3);
        }
        errors++;
      }
    }
    if (data.subject !== void 0) {
      if (!validate68(data.subject, { instancePath: instancePath + "/subject", parentData: data, parentDataProperty: "subject", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate68.errors : vErrors.concat(validate68.errors);
        errors = vErrors.length;
      }
    }
  } else {
    const err4 = { instancePath, schemaPath: "#/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err4];
    } else {
      vErrors.push(err4);
    }
    errors++;
  }
  validate60.errors = vErrors;
  return errors === 0;
}
validate60.evaluated = { "props": { "change": true, "rootDigest": true, "subject": true }, "dynamicProps": false, "dynamicItems": false };
function validate59(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate59.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.entries === void 0) {
      const err0 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "entries" }, message: "must have required property 'entries'" };
      if (vErrors === null) {
        vErrors = [err0];
      } else {
        vErrors.push(err0);
      }
      errors++;
    }
    if (data.nextOffset === void 0) {
      const err1 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "nextOffset" }, message: "must have required property 'nextOffset'" };
      if (vErrors === null) {
        vErrors = [err1];
      } else {
        vErrors.push(err1);
      }
      errors++;
    }
    if (data.entries !== void 0) {
      let data0 = data.entries;
      if (Array.isArray(data0)) {
        const len0 = data0.length;
        for (let i0 = 0; i0 < len0; i0++) {
          if (!validate60(data0[i0], { instancePath: instancePath + "/entries/" + i0, parentData: data0, parentDataProperty: i0, rootData, dynamicAnchors })) {
            vErrors = vErrors === null ? validate60.errors : vErrors.concat(validate60.errors);
            errors = vErrors.length;
          }
        }
      } else {
        const err2 = { instancePath: instancePath + "/entries", schemaPath: "#/properties/entries/type", keyword: "type", params: { type: "array" }, message: "must be array" };
        if (vErrors === null) {
          vErrors = [err2];
        } else {
          vErrors.push(err2);
        }
        errors++;
      }
    }
    if (data.nextOffset !== void 0) {
      let data2 = data.nextOffset;
      if (!(typeof data2 == "number" && (!(data2 % 1) && !isNaN(data2))) && data2 !== null) {
        const err3 = { instancePath: instancePath + "/nextOffset", schemaPath: "#/properties/nextOffset/type", keyword: "type", params: { type: schema23.properties.nextOffset.type }, message: "must be integer,null" };
        if (vErrors === null) {
          vErrors = [err3];
        } else {
          vErrors.push(err3);
        }
        errors++;
      }
      if (typeof data2 == "number") {
        if (data2 < 0 || isNaN(data2)) {
          const err4 = { instancePath: instancePath + "/nextOffset", schemaPath: "#/properties/nextOffset/minimum", keyword: "minimum", params: { comparison: ">=", limit: 0 }, message: "must be >= 0" };
          if (vErrors === null) {
            vErrors = [err4];
          } else {
            vErrors.push(err4);
          }
          errors++;
        }
        if (!formats4.validate(data2)) {
          const err5 = { instancePath: instancePath + "/nextOffset", schemaPath: "#/properties/nextOffset/format", keyword: "format", params: { format: "uint32" }, message: 'must match format "uint32"' };
          if (vErrors === null) {
            vErrors = [err5];
          } else {
            vErrors.push(err5);
          }
          errors++;
        }
      }
    }
  } else {
    const err6 = { instancePath, schemaPath: "#/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err6];
    } else {
      vErrors.push(err6);
    }
    errors++;
  }
  validate59.errors = vErrors;
  return errors === 0;
}
validate59.evaluated = { "props": { "entries": true, "nextOffset": true }, "dynamicProps": false, "dynamicItems": false };
var schema36 = { "description": "One bounded page of the complete current impact partition.", "properties": { "entries": { "items": { "$ref": "#/$defs/Impact" }, "type": "array" }, "nextOffset": { "format": "uint32", "minimum": 0, "type": ["integer", "null"] } }, "required": ["entries", "nextOffset"], "type": "object" };
var schema38 = { "description": "Closed reachability classification.", "enum": ["affected", "unaffected"], "type": "string" };
function validate99(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate99.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  if (typeof data !== "string") {
    const err0 = { instancePath, schemaPath: "#/type", keyword: "type", params: { type: "string" }, message: "must be string" };
    if (vErrors === null) {
      vErrors = [err0];
    } else {
      vErrors.push(err0);
    }
    errors++;
  }
  if (!(data === "affected" || data === "unaffected")) {
    const err1 = { instancePath, schemaPath: "#/enum", keyword: "enum", params: { allowedValues: schema38.enum }, message: "must be equal to one of the allowed values" };
    if (vErrors === null) {
      vErrors = [err1];
    } else {
      vErrors.push(err1);
    }
    errors++;
  }
  validate99.errors = vErrors;
  return errors === 0;
}
validate99.evaluated = { "dynamicProps": false, "dynamicItems": false };
function validate98(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate98.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.subject === void 0) {
      const err0 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "subject" }, message: "must have required property 'subject'" };
      if (vErrors === null) {
        vErrors = [err0];
      } else {
        vErrors.push(err0);
      }
      errors++;
    }
    if (data.state === void 0) {
      const err1 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "state" }, message: "must have required property 'state'" };
      if (vErrors === null) {
        vErrors = [err1];
      } else {
        vErrors.push(err1);
      }
      errors++;
    }
    if (data.contributingRootCount === void 0) {
      const err2 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "contributingRootCount" }, message: "must have required property 'contributingRootCount'" };
      if (vErrors === null) {
        vErrors = [err2];
      } else {
        vErrors.push(err2);
      }
      errors++;
    }
    if (data.contributingRootCount !== void 0) {
      let data0 = data.contributingRootCount;
      if (!(typeof data0 == "number" && (!(data0 % 1) && !isNaN(data0)))) {
        const err3 = { instancePath: instancePath + "/contributingRootCount", schemaPath: "#/properties/contributingRootCount/type", keyword: "type", params: { type: "integer" }, message: "must be integer" };
        if (vErrors === null) {
          vErrors = [err3];
        } else {
          vErrors.push(err3);
        }
        errors++;
      }
      if (typeof data0 == "number") {
        if (data0 < 0 || isNaN(data0)) {
          const err4 = { instancePath: instancePath + "/contributingRootCount", schemaPath: "#/properties/contributingRootCount/minimum", keyword: "minimum", params: { comparison: ">=", limit: 0 }, message: "must be >= 0" };
          if (vErrors === null) {
            vErrors = [err4];
          } else {
            vErrors.push(err4);
          }
          errors++;
        }
        if (!formats4.validate(data0)) {
          const err5 = { instancePath: instancePath + "/contributingRootCount", schemaPath: "#/properties/contributingRootCount/format", keyword: "format", params: { format: "uint32" }, message: 'must match format "uint32"' };
          if (vErrors === null) {
            vErrors = [err5];
          } else {
            vErrors.push(err5);
          }
          errors++;
        }
      }
    }
    if (data.state !== void 0) {
      if (!validate99(data.state, { instancePath: instancePath + "/state", parentData: data, parentDataProperty: "state", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate99.errors : vErrors.concat(validate99.errors);
        errors = vErrors.length;
      }
    }
    if (data.subject !== void 0) {
      if (!validate68(data.subject, { instancePath: instancePath + "/subject", parentData: data, parentDataProperty: "subject", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate68.errors : vErrors.concat(validate68.errors);
        errors = vErrors.length;
      }
    }
  } else {
    const err6 = { instancePath, schemaPath: "#/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err6];
    } else {
      vErrors.push(err6);
    }
    errors++;
  }
  validate98.errors = vErrors;
  return errors === 0;
}
validate98.evaluated = { "props": { "contributingRootCount": true, "state": true, "subject": true }, "dynamicProps": false, "dynamicItems": false };
function validate97(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate97.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.entries === void 0) {
      const err0 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "entries" }, message: "must have required property 'entries'" };
      if (vErrors === null) {
        vErrors = [err0];
      } else {
        vErrors.push(err0);
      }
      errors++;
    }
    if (data.nextOffset === void 0) {
      const err1 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "nextOffset" }, message: "must have required property 'nextOffset'" };
      if (vErrors === null) {
        vErrors = [err1];
      } else {
        vErrors.push(err1);
      }
      errors++;
    }
    if (data.entries !== void 0) {
      let data0 = data.entries;
      if (Array.isArray(data0)) {
        const len0 = data0.length;
        for (let i0 = 0; i0 < len0; i0++) {
          if (!validate98(data0[i0], { instancePath: instancePath + "/entries/" + i0, parentData: data0, parentDataProperty: i0, rootData, dynamicAnchors })) {
            vErrors = vErrors === null ? validate98.errors : vErrors.concat(validate98.errors);
            errors = vErrors.length;
          }
        }
      } else {
        const err2 = { instancePath: instancePath + "/entries", schemaPath: "#/properties/entries/type", keyword: "type", params: { type: "array" }, message: "must be array" };
        if (vErrors === null) {
          vErrors = [err2];
        } else {
          vErrors.push(err2);
        }
        errors++;
      }
    }
    if (data.nextOffset !== void 0) {
      let data2 = data.nextOffset;
      if (!(typeof data2 == "number" && (!(data2 % 1) && !isNaN(data2))) && data2 !== null) {
        const err3 = { instancePath: instancePath + "/nextOffset", schemaPath: "#/properties/nextOffset/type", keyword: "type", params: { type: schema36.properties.nextOffset.type }, message: "must be integer,null" };
        if (vErrors === null) {
          vErrors = [err3];
        } else {
          vErrors.push(err3);
        }
        errors++;
      }
      if (typeof data2 == "number") {
        if (data2 < 0 || isNaN(data2)) {
          const err4 = { instancePath: instancePath + "/nextOffset", schemaPath: "#/properties/nextOffset/minimum", keyword: "minimum", params: { comparison: ">=", limit: 0 }, message: "must be >= 0" };
          if (vErrors === null) {
            vErrors = [err4];
          } else {
            vErrors.push(err4);
          }
          errors++;
        }
        if (!formats4.validate(data2)) {
          const err5 = { instancePath: instancePath + "/nextOffset", schemaPath: "#/properties/nextOffset/format", keyword: "format", params: { format: "uint32" }, message: 'must match format "uint32"' };
          if (vErrors === null) {
            vErrors = [err5];
          } else {
            vErrors.push(err5);
          }
          errors++;
        }
      }
    }
  } else {
    const err6 = { instancePath, schemaPath: "#/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err6];
    } else {
      vErrors.push(err6);
    }
    errors++;
  }
  validate97.errors = vErrors;
  return errors === 0;
}
validate97.evaluated = { "props": { "entries": true, "nextOffset": true }, "dynamicProps": false, "dynamicItems": false };
var schema41 = { "description": "Proof DAG selected for one causal witness.", "enum": ["predecessor", "current"], "type": "string" };
function validate108(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate108.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  if (typeof data !== "string") {
    const err0 = { instancePath, schemaPath: "#/type", keyword: "type", params: { type: "string" }, message: "must be string" };
    if (vErrors === null) {
      vErrors = [err0];
    } else {
      vErrors.push(err0);
    }
    errors++;
  }
  if (!(data === "predecessor" || data === "current")) {
    const err1 = { instancePath, schemaPath: "#/enum", keyword: "enum", params: { allowedValues: schema41.enum }, message: "must be equal to one of the allowed values" };
    if (vErrors === null) {
      vErrors = [err1];
    } else {
      vErrors.push(err1);
    }
    errors++;
  }
  validate108.errors = vErrors;
  return errors === 0;
}
validate108.evaluated = { "dynamicProps": false, "dynamicItems": false };
function validate113(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate113.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  const _errs0 = errors;
  let valid0 = false;
  let passing0 = null;
  const _errs1 = errors;
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.kind === void 0) {
      const err0 = { instancePath, schemaPath: "#/oneOf/0/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
      if (vErrors === null) {
        vErrors = [err0];
      } else {
        vErrors.push(err0);
      }
      errors++;
    }
    if (data.sourceId === void 0) {
      const err1 = { instancePath, schemaPath: "#/oneOf/0/required", keyword: "required", params: { missingProperty: "sourceId" }, message: "must have required property 'sourceId'" };
      if (vErrors === null) {
        vErrors = [err1];
      } else {
        vErrors.push(err1);
      }
      errors++;
    }
    if (data.kind !== void 0) {
      let data0 = data.kind;
      if (typeof data0 !== "string") {
        const err2 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/0/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err2];
        } else {
          vErrors.push(err2);
        }
        errors++;
      }
      if ("source" !== data0) {
        const err3 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/0/properties/kind/const", keyword: "const", params: { allowedValue: "source" }, message: "must be equal to constant" };
        if (vErrors === null) {
          vErrors = [err3];
        } else {
          vErrors.push(err3);
        }
        errors++;
      }
    }
    if (data.sourceId !== void 0) {
      if (typeof data.sourceId !== "string") {
        const err4 = { instancePath: instancePath + "/sourceId", schemaPath: "#/oneOf/0/properties/sourceId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err4];
        } else {
          vErrors.push(err4);
        }
        errors++;
      }
    }
  } else {
    const err5 = { instancePath, schemaPath: "#/oneOf/0/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err5];
    } else {
      vErrors.push(err5);
    }
    errors++;
  }
  var _valid0 = _errs1 === errors;
  if (_valid0) {
    valid0 = true;
    passing0 = 0;
    var props0 = {};
    props0.kind = true;
    props0.sourceId = true;
  }
  const _errs7 = errors;
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.kind === void 0) {
      const err6 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
      if (vErrors === null) {
        vErrors = [err6];
      } else {
        vErrors.push(err6);
      }
      errors++;
    }
    if (data.moduleId === void 0) {
      const err7 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "moduleId" }, message: "must have required property 'moduleId'" };
      if (vErrors === null) {
        vErrors = [err7];
      } else {
        vErrors.push(err7);
      }
      errors++;
    }
    if (data.releaseId === void 0) {
      const err8 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "releaseId" }, message: "must have required property 'releaseId'" };
      if (vErrors === null) {
        vErrors = [err8];
      } else {
        vErrors.push(err8);
      }
      errors++;
    }
    if (data.moduleDigest === void 0) {
      const err9 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "moduleDigest" }, message: "must have required property 'moduleDigest'" };
      if (vErrors === null) {
        vErrors = [err9];
      } else {
        vErrors.push(err9);
      }
      errors++;
    }
    if (data.kind !== void 0) {
      let data2 = data.kind;
      if (typeof data2 !== "string") {
        const err10 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/1/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err10];
        } else {
          vErrors.push(err10);
        }
        errors++;
      }
      if ("module-release" !== data2) {
        const err11 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/1/properties/kind/const", keyword: "const", params: { allowedValue: "module-release" }, message: "must be equal to constant" };
        if (vErrors === null) {
          vErrors = [err11];
        } else {
          vErrors.push(err11);
        }
        errors++;
      }
    }
    if (data.moduleDigest !== void 0) {
      if (typeof data.moduleDigest !== "string") {
        const err12 = { instancePath: instancePath + "/moduleDigest", schemaPath: "#/oneOf/1/properties/moduleDigest/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err12];
        } else {
          vErrors.push(err12);
        }
        errors++;
      }
    }
    if (data.moduleId !== void 0) {
      if (typeof data.moduleId !== "string") {
        const err13 = { instancePath: instancePath + "/moduleId", schemaPath: "#/oneOf/1/properties/moduleId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err13];
        } else {
          vErrors.push(err13);
        }
        errors++;
      }
    }
    if (data.releaseId !== void 0) {
      if (typeof data.releaseId !== "string") {
        const err14 = { instancePath: instancePath + "/releaseId", schemaPath: "#/oneOf/1/properties/releaseId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err14];
        } else {
          vErrors.push(err14);
        }
        errors++;
      }
    }
  } else {
    const err15 = { instancePath, schemaPath: "#/oneOf/1/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err15];
    } else {
      vErrors.push(err15);
    }
    errors++;
  }
  var _valid0 = _errs7 === errors;
  if (_valid0 && valid0) {
    valid0 = false;
    passing0 = [passing0, 1];
  } else {
    if (_valid0) {
      valid0 = true;
      passing0 = 1;
      if (props0 !== true) {
        props0 = props0 || {};
        props0.kind = true;
        props0.moduleDigest = true;
        props0.moduleId = true;
        props0.releaseId = true;
      }
    }
    const _errs17 = errors;
    if (data && typeof data == "object" && !Array.isArray(data)) {
      if (data.kind === void 0) {
        const err16 = { instancePath, schemaPath: "#/oneOf/2/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
        if (vErrors === null) {
          vErrors = [err16];
        } else {
          vErrors.push(err16);
        }
        errors++;
      }
      if (data.importerModuleId === void 0) {
        const err17 = { instancePath, schemaPath: "#/oneOf/2/required", keyword: "required", params: { missingProperty: "importerModuleId" }, message: "must have required property 'importerModuleId'" };
        if (vErrors === null) {
          vErrors = [err17];
        } else {
          vErrors.push(err17);
        }
        errors++;
      }
      if (data.upstreamModuleId === void 0) {
        const err18 = { instancePath, schemaPath: "#/oneOf/2/required", keyword: "required", params: { missingProperty: "upstreamModuleId" }, message: "must have required property 'upstreamModuleId'" };
        if (vErrors === null) {
          vErrors = [err18];
        } else {
          vErrors.push(err18);
        }
        errors++;
      }
      if (data.upstreamReleaseId === void 0) {
        const err19 = { instancePath, schemaPath: "#/oneOf/2/required", keyword: "required", params: { missingProperty: "upstreamReleaseId" }, message: "must have required property 'upstreamReleaseId'" };
        if (vErrors === null) {
          vErrors = [err19];
        } else {
          vErrors.push(err19);
        }
        errors++;
      }
      if (data.exportId === void 0) {
        const err20 = { instancePath, schemaPath: "#/oneOf/2/required", keyword: "required", params: { missingProperty: "exportId" }, message: "must have required property 'exportId'" };
        if (vErrors === null) {
          vErrors = [err20];
        } else {
          vErrors.push(err20);
        }
        errors++;
      }
      if (data.sourceNamespaceId === void 0) {
        const err21 = { instancePath, schemaPath: "#/oneOf/2/required", keyword: "required", params: { missingProperty: "sourceNamespaceId" }, message: "must have required property 'sourceNamespaceId'" };
        if (vErrors === null) {
          vErrors = [err21];
        } else {
          vErrors.push(err21);
        }
        errors++;
      }
      if (data.localNamespaceId === void 0) {
        const err22 = { instancePath, schemaPath: "#/oneOf/2/required", keyword: "required", params: { missingProperty: "localNamespaceId" }, message: "must have required property 'localNamespaceId'" };
        if (vErrors === null) {
          vErrors = [err22];
        } else {
          vErrors.push(err22);
        }
        errors++;
      }
      if (data.exportId !== void 0) {
        if (typeof data.exportId !== "string") {
          const err23 = { instancePath: instancePath + "/exportId", schemaPath: "#/oneOf/2/properties/exportId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
          if (vErrors === null) {
            vErrors = [err23];
          } else {
            vErrors.push(err23);
          }
          errors++;
        }
      }
      if (data.importerModuleId !== void 0) {
        if (typeof data.importerModuleId !== "string") {
          const err24 = { instancePath: instancePath + "/importerModuleId", schemaPath: "#/oneOf/2/properties/importerModuleId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
          if (vErrors === null) {
            vErrors = [err24];
          } else {
            vErrors.push(err24);
          }
          errors++;
        }
      }
      if (data.kind !== void 0) {
        let data8 = data.kind;
        if (typeof data8 !== "string") {
          const err25 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/2/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
          if (vErrors === null) {
            vErrors = [err25];
          } else {
            vErrors.push(err25);
          }
          errors++;
        }
        if ("import" !== data8) {
          const err26 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/2/properties/kind/const", keyword: "const", params: { allowedValue: "import" }, message: "must be equal to constant" };
          if (vErrors === null) {
            vErrors = [err26];
          } else {
            vErrors.push(err26);
          }
          errors++;
        }
      }
      if (data.localNamespaceId !== void 0) {
        if (typeof data.localNamespaceId !== "string") {
          const err27 = { instancePath: instancePath + "/localNamespaceId", schemaPath: "#/oneOf/2/properties/localNamespaceId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
          if (vErrors === null) {
            vErrors = [err27];
          } else {
            vErrors.push(err27);
          }
          errors++;
        }
      }
      if (data.sourceNamespaceId !== void 0) {
        if (typeof data.sourceNamespaceId !== "string") {
          const err28 = { instancePath: instancePath + "/sourceNamespaceId", schemaPath: "#/oneOf/2/properties/sourceNamespaceId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
          if (vErrors === null) {
            vErrors = [err28];
          } else {
            vErrors.push(err28);
          }
          errors++;
        }
      }
      if (data.upstreamModuleId !== void 0) {
        if (typeof data.upstreamModuleId !== "string") {
          const err29 = { instancePath: instancePath + "/upstreamModuleId", schemaPath: "#/oneOf/2/properties/upstreamModuleId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
          if (vErrors === null) {
            vErrors = [err29];
          } else {
            vErrors.push(err29);
          }
          errors++;
        }
      }
      if (data.upstreamReleaseId !== void 0) {
        if (typeof data.upstreamReleaseId !== "string") {
          const err30 = { instancePath: instancePath + "/upstreamReleaseId", schemaPath: "#/oneOf/2/properties/upstreamReleaseId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
          if (vErrors === null) {
            vErrors = [err30];
          } else {
            vErrors.push(err30);
          }
          errors++;
        }
      }
    } else {
      const err31 = { instancePath, schemaPath: "#/oneOf/2/type", keyword: "type", params: { type: "object" }, message: "must be object" };
      if (vErrors === null) {
        vErrors = [err31];
      } else {
        vErrors.push(err31);
      }
      errors++;
    }
    var _valid0 = _errs17 === errors;
    if (_valid0 && valid0) {
      valid0 = false;
      passing0 = [passing0, 2];
    } else {
      if (_valid0) {
        valid0 = true;
        passing0 = 2;
        if (props0 !== true) {
          props0 = props0 || {};
          props0.exportId = true;
          props0.importerModuleId = true;
          props0.kind = true;
          props0.localNamespaceId = true;
          props0.sourceNamespaceId = true;
          props0.upstreamModuleId = true;
          props0.upstreamReleaseId = true;
        }
      }
      const _errs33 = errors;
      if (data && typeof data == "object" && !Array.isArray(data)) {
        if (data.kind === void 0) {
          const err32 = { instancePath, schemaPath: "#/oneOf/3/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
          if (vErrors === null) {
            vErrors = [err32];
          } else {
            vErrors.push(err32);
          }
          errors++;
        }
        if (data.ownerModuleId === void 0) {
          const err33 = { instancePath, schemaPath: "#/oneOf/3/required", keyword: "required", params: { missingProperty: "ownerModuleId" }, message: "must have required property 'ownerModuleId'" };
          if (vErrors === null) {
            vErrors = [err33];
          } else {
            vErrors.push(err33);
          }
          errors++;
        }
        if (data.extensionNamespaceId === void 0) {
          const err34 = { instancePath, schemaPath: "#/oneOf/3/required", keyword: "required", params: { missingProperty: "extensionNamespaceId" }, message: "must have required property 'extensionNamespaceId'" };
          if (vErrors === null) {
            vErrors = [err34];
          } else {
            vErrors.push(err34);
          }
          errors++;
        }
        if (data.targetNamespaceId === void 0) {
          const err35 = { instancePath, schemaPath: "#/oneOf/3/required", keyword: "required", params: { missingProperty: "targetNamespaceId" }, message: "must have required property 'targetNamespaceId'" };
          if (vErrors === null) {
            vErrors = [err35];
          } else {
            vErrors.push(err35);
          }
          errors++;
        }
        if (data.targetDeclarationId === void 0) {
          const err36 = { instancePath, schemaPath: "#/oneOf/3/required", keyword: "required", params: { missingProperty: "targetDeclarationId" }, message: "must have required property 'targetDeclarationId'" };
          if (vErrors === null) {
            vErrors = [err36];
          } else {
            vErrors.push(err36);
          }
          errors++;
        }
        if (data.extensionNamespaceId !== void 0) {
          if (typeof data.extensionNamespaceId !== "string") {
            const err37 = { instancePath: instancePath + "/extensionNamespaceId", schemaPath: "#/oneOf/3/properties/extensionNamespaceId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
            if (vErrors === null) {
              vErrors = [err37];
            } else {
              vErrors.push(err37);
            }
            errors++;
          }
        }
        if (data.kind !== void 0) {
          let data14 = data.kind;
          if (typeof data14 !== "string") {
            const err38 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/3/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
            if (vErrors === null) {
              vErrors = [err38];
            } else {
              vErrors.push(err38);
            }
            errors++;
          }
          if ("extension" !== data14) {
            const err39 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/3/properties/kind/const", keyword: "const", params: { allowedValue: "extension" }, message: "must be equal to constant" };
            if (vErrors === null) {
              vErrors = [err39];
            } else {
              vErrors.push(err39);
            }
            errors++;
          }
        }
        if (data.ownerModuleId !== void 0) {
          if (typeof data.ownerModuleId !== "string") {
            const err40 = { instancePath: instancePath + "/ownerModuleId", schemaPath: "#/oneOf/3/properties/ownerModuleId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
            if (vErrors === null) {
              vErrors = [err40];
            } else {
              vErrors.push(err40);
            }
            errors++;
          }
        }
        if (data.targetDeclarationId !== void 0) {
          if (typeof data.targetDeclarationId !== "string") {
            const err41 = { instancePath: instancePath + "/targetDeclarationId", schemaPath: "#/oneOf/3/properties/targetDeclarationId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
            if (vErrors === null) {
              vErrors = [err41];
            } else {
              vErrors.push(err41);
            }
            errors++;
          }
        }
        if (data.targetNamespaceId !== void 0) {
          if (typeof data.targetNamespaceId !== "string") {
            const err42 = { instancePath: instancePath + "/targetNamespaceId", schemaPath: "#/oneOf/3/properties/targetNamespaceId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
            if (vErrors === null) {
              vErrors = [err42];
            } else {
              vErrors.push(err42);
            }
            errors++;
          }
        }
      } else {
        const err43 = { instancePath, schemaPath: "#/oneOf/3/type", keyword: "type", params: { type: "object" }, message: "must be object" };
        if (vErrors === null) {
          vErrors = [err43];
        } else {
          vErrors.push(err43);
        }
        errors++;
      }
      var _valid0 = _errs33 === errors;
      if (_valid0 && valid0) {
        valid0 = false;
        passing0 = [passing0, 3];
      } else {
        if (_valid0) {
          valid0 = true;
          passing0 = 3;
          if (props0 !== true) {
            props0 = props0 || {};
            props0.extensionNamespaceId = true;
            props0.kind = true;
            props0.ownerModuleId = true;
            props0.targetDeclarationId = true;
            props0.targetNamespaceId = true;
          }
        }
        const _errs45 = errors;
        if (data && typeof data == "object" && !Array.isArray(data)) {
          if (data.kind === void 0) {
            const err44 = { instancePath, schemaPath: "#/oneOf/4/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
            if (vErrors === null) {
              vErrors = [err44];
            } else {
              vErrors.push(err44);
            }
            errors++;
          }
          if (data.kind !== void 0) {
            let data18 = data.kind;
            if (typeof data18 !== "string") {
              const err45 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/4/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
              if (vErrors === null) {
                vErrors = [err45];
              } else {
                vErrors.push(err45);
              }
              errors++;
            }
            if ("compilation" !== data18) {
              const err46 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/4/properties/kind/const", keyword: "const", params: { allowedValue: "compilation" }, message: "must be equal to constant" };
              if (vErrors === null) {
                vErrors = [err46];
              } else {
                vErrors.push(err46);
              }
              errors++;
            }
          }
        } else {
          const err47 = { instancePath, schemaPath: "#/oneOf/4/type", keyword: "type", params: { type: "object" }, message: "must be object" };
          if (vErrors === null) {
            vErrors = [err47];
          } else {
            vErrors.push(err47);
          }
          errors++;
        }
        var _valid0 = _errs45 === errors;
        if (_valid0 && valid0) {
          valid0 = false;
          passing0 = [passing0, 4];
        } else {
          if (_valid0) {
            valid0 = true;
            passing0 = 4;
            if (props0 !== true) {
              props0 = props0 || {};
              props0.kind = true;
            }
          }
        }
      }
    }
  }
  if (!valid0) {
    const err48 = { instancePath, schemaPath: "#/oneOf", keyword: "oneOf", params: { passingSchemas: passing0 }, message: "must match exactly one schema in oneOf" };
    if (vErrors === null) {
      vErrors = [err48];
    } else {
      vErrors.push(err48);
    }
    errors++;
  } else {
    errors = _errs0;
    if (vErrors !== null) {
      if (_errs0) {
        vErrors.length = _errs0;
      } else {
        vErrors = null;
      }
    }
  }
  validate113.errors = vErrors;
  evaluated0.props = props0;
  return errors === 0;
}
validate113.evaluated = { "dynamicProps": true, "dynamicItems": false };
var schema44 = { "description": "Deterministic compiler rule that produced one causal step.", "enum": ["source-admission", "component-admission", "module-sealing", "declaration-linking", "import-linking", "extension-application", "policy-composition", "responsibility-lowering", "organizational-translation", "experience-composition", "resource-selection", "resolution-locking", "universe-locking", "semantic-ir-sealing"], "type": "string" };
function validate115(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate115.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  if (typeof data !== "string") {
    const err0 = { instancePath, schemaPath: "#/type", keyword: "type", params: { type: "string" }, message: "must be string" };
    if (vErrors === null) {
      vErrors = [err0];
    } else {
      vErrors.push(err0);
    }
    errors++;
  }
  if (!(data === "source-admission" || data === "component-admission" || data === "module-sealing" || data === "declaration-linking" || data === "import-linking" || data === "extension-application" || data === "policy-composition" || data === "responsibility-lowering" || data === "organizational-translation" || data === "experience-composition" || data === "resource-selection" || data === "resolution-locking" || data === "universe-locking" || data === "semantic-ir-sealing")) {
    const err1 = { instancePath, schemaPath: "#/enum", keyword: "enum", params: { allowedValues: schema44.enum }, message: "must be equal to one of the allowed values" };
    if (vErrors === null) {
      vErrors = [err1];
    } else {
      vErrors.push(err1);
    }
    errors++;
  }
  validate115.errors = vErrors;
  return errors === 0;
}
validate115.evaluated = { "dynamicProps": false, "dynamicItems": false };
function validate111(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate111.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.nodeDigest === void 0) {
      const err0 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "nodeDigest" }, message: "must have required property 'nodeDigest'" };
      if (vErrors === null) {
        vErrors = [err0];
      } else {
        vErrors.push(err0);
      }
      errors++;
    }
    if (data.subject === void 0) {
      const err1 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "subject" }, message: "must have required property 'subject'" };
      if (vErrors === null) {
        vErrors = [err1];
      } else {
        vErrors.push(err1);
      }
      errors++;
    }
    if (data.provenance === void 0) {
      const err2 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "provenance" }, message: "must have required property 'provenance'" };
      if (vErrors === null) {
        vErrors = [err2];
      } else {
        vErrors.push(err2);
      }
      errors++;
    }
    if (data.rule === void 0) {
      const err3 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "rule" }, message: "must have required property 'rule'" };
      if (vErrors === null) {
        vErrors = [err3];
      } else {
        vErrors.push(err3);
      }
      errors++;
    }
    if (data.fact === void 0) {
      const err4 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "fact" }, message: "must have required property 'fact'" };
      if (vErrors === null) {
        vErrors = [err4];
      } else {
        vErrors.push(err4);
      }
      errors++;
    }
    if (data.fact !== void 0) {
      if (!validate62(data.fact, { instancePath: instancePath + "/fact", parentData: data, parentDataProperty: "fact", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate62.errors : vErrors.concat(validate62.errors);
        errors = vErrors.length;
      }
    }
    if (data.nodeDigest !== void 0) {
      if (typeof data.nodeDigest !== "string") {
        const err5 = { instancePath: instancePath + "/nodeDigest", schemaPath: "#/properties/nodeDigest/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err5];
        } else {
          vErrors.push(err5);
        }
        errors++;
      }
    }
    if (data.provenance !== void 0) {
      if (!validate113(data.provenance, { instancePath: instancePath + "/provenance", parentData: data, parentDataProperty: "provenance", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate113.errors : vErrors.concat(validate113.errors);
        errors = vErrors.length;
      }
    }
    if (data.rule !== void 0) {
      if (!validate115(data.rule, { instancePath: instancePath + "/rule", parentData: data, parentDataProperty: "rule", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate115.errors : vErrors.concat(validate115.errors);
        errors = vErrors.length;
      }
    }
    if (data.subject !== void 0) {
      if (!validate68(data.subject, { instancePath: instancePath + "/subject", parentData: data, parentDataProperty: "subject", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate68.errors : vErrors.concat(validate68.errors);
        errors = vErrors.length;
      }
    }
  } else {
    const err6 = { instancePath, schemaPath: "#/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err6];
    } else {
      vErrors.push(err6);
    }
    errors++;
  }
  validate111.errors = vErrors;
  return errors === 0;
}
validate111.evaluated = { "props": { "fact": true, "nodeDigest": true, "provenance": true, "rule": true, "subject": true }, "dynamicProps": false, "dynamicItems": false };
function validate107(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate107.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.root === void 0) {
      const err0 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "root" }, message: "must have required property 'root'" };
      if (vErrors === null) {
        vErrors = [err0];
      } else {
        vErrors.push(err0);
      }
      errors++;
    }
    if (data.frontier === void 0) {
      const err1 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "frontier" }, message: "must have required property 'frontier'" };
      if (vErrors === null) {
        vErrors = [err1];
      } else {
        vErrors.push(err1);
      }
      errors++;
    }
    if (data.steps === void 0) {
      const err2 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "steps" }, message: "must have required property 'steps'" };
      if (vErrors === null) {
        vErrors = [err2];
      } else {
        vErrors.push(err2);
      }
      errors++;
    }
    if (data.frontier !== void 0) {
      if (!validate108(data.frontier, { instancePath: instancePath + "/frontier", parentData: data, parentDataProperty: "frontier", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate108.errors : vErrors.concat(validate108.errors);
        errors = vErrors.length;
      }
    }
    if (data.root !== void 0) {
      if (!validate60(data.root, { instancePath: instancePath + "/root", parentData: data, parentDataProperty: "root", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate60.errors : vErrors.concat(validate60.errors);
        errors = vErrors.length;
      }
    }
    if (data.steps !== void 0) {
      let data2 = data.steps;
      if (Array.isArray(data2)) {
        const len0 = data2.length;
        for (let i0 = 0; i0 < len0; i0++) {
          if (!validate111(data2[i0], { instancePath: instancePath + "/steps/" + i0, parentData: data2, parentDataProperty: i0, rootData, dynamicAnchors })) {
            vErrors = vErrors === null ? validate111.errors : vErrors.concat(validate111.errors);
            errors = vErrors.length;
          }
        }
      } else {
        const err3 = { instancePath: instancePath + "/steps", schemaPath: "#/properties/steps/type", keyword: "type", params: { type: "array" }, message: "must be array" };
        if (vErrors === null) {
          vErrors = [err3];
        } else {
          vErrors.push(err3);
        }
        errors++;
      }
    }
  } else {
    const err4 = { instancePath, schemaPath: "#/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err4];
    } else {
      vErrors.push(err4);
    }
    errors++;
  }
  validate107.errors = vErrors;
  return errors === 0;
}
validate107.evaluated = { "props": { "frontier": true, "root": true, "steps": true }, "dynamicProps": false, "dynamicItems": false };
var schema45 = { "description": "Closed explanation state.", "enum": ["not-present", "unaffected", "affected"], "type": "string" };
function validate120(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate120.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  if (typeof data !== "string") {
    const err0 = { instancePath, schemaPath: "#/type", keyword: "type", params: { type: "string" }, message: "must be string" };
    if (vErrors === null) {
      vErrors = [err0];
    } else {
      vErrors.push(err0);
    }
    errors++;
  }
  if (!(data === "not-present" || data === "unaffected" || data === "affected")) {
    const err1 = { instancePath, schemaPath: "#/enum", keyword: "enum", params: { allowedValues: schema45.enum }, message: "must be equal to one of the allowed values" };
    if (vErrors === null) {
      vErrors = [err1];
    } else {
      vErrors.push(err1);
    }
    errors++;
  }
  validate120.errors = vErrors;
  return errors === 0;
}
validate120.evaluated = { "dynamicProps": false, "dynamicItems": false };
function validate106(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate106.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.subject === void 0) {
      const err0 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "subject" }, message: "must have required property 'subject'" };
      if (vErrors === null) {
        vErrors = [err0];
      } else {
        vErrors.push(err0);
      }
      errors++;
    }
    if (data.state === void 0) {
      const err1 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "state" }, message: "must have required property 'state'" };
      if (vErrors === null) {
        vErrors = [err1];
      } else {
        vErrors.push(err1);
      }
      errors++;
    }
    if (data.causal !== void 0) {
      if (!validate107(data.causal, { instancePath: instancePath + "/causal", parentData: data, parentDataProperty: "causal", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate107.errors : vErrors.concat(validate107.errors);
        errors = vErrors.length;
      }
    }
    if (data.state !== void 0) {
      if (!validate120(data.state, { instancePath: instancePath + "/state", parentData: data, parentDataProperty: "state", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate120.errors : vErrors.concat(validate120.errors);
        errors = vErrors.length;
      }
    }
    if (data.subject !== void 0) {
      if (!validate68(data.subject, { instancePath: instancePath + "/subject", parentData: data, parentDataProperty: "subject", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate68.errors : vErrors.concat(validate68.errors);
        errors = vErrors.length;
      }
    }
  } else {
    const err2 = { instancePath, schemaPath: "#/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err2];
    } else {
      vErrors.push(err2);
    }
    errors++;
  }
  validate106.errors = vErrors;
  return errors === 0;
}
validate106.evaluated = { "props": { "causal": true, "state": true, "subject": true }, "dynamicProps": false, "dynamicItems": false };
var formats42 = { ...require_formats().fullFormats, ...(init_number_formats(), __toCommonJS(number_formats_exports)).numberFormats }.uint16;
function validate127(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate127.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  const _errs0 = errors;
  let valid0 = false;
  let passing0 = null;
  const _errs1 = errors;
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.kind === void 0) {
      const err0 = { instancePath, schemaPath: "#/oneOf/0/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
      if (vErrors === null) {
        vErrors = [err0];
      } else {
        vErrors.push(err0);
      }
      errors++;
    }
    if (data.value === void 0) {
      const err1 = { instancePath, schemaPath: "#/oneOf/0/required", keyword: "required", params: { missingProperty: "value" }, message: "must have required property 'value'" };
      if (vErrors === null) {
        vErrors = [err1];
      } else {
        vErrors.push(err1);
      }
      errors++;
    }
    if (data.kind !== void 0) {
      let data0 = data.kind;
      if (typeof data0 !== "string") {
        const err2 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/0/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err2];
        } else {
          vErrors.push(err2);
        }
        errors++;
      }
      if ("field" !== data0) {
        const err3 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/0/properties/kind/const", keyword: "const", params: { allowedValue: "field" }, message: "must be equal to constant" };
        if (vErrors === null) {
          vErrors = [err3];
        } else {
          vErrors.push(err3);
        }
        errors++;
      }
    }
    if (data.value !== void 0) {
      if (typeof data.value !== "string") {
        const err4 = { instancePath: instancePath + "/value", schemaPath: "#/oneOf/0/properties/value/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err4];
        } else {
          vErrors.push(err4);
        }
        errors++;
      }
    }
  } else {
    const err5 = { instancePath, schemaPath: "#/oneOf/0/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err5];
    } else {
      vErrors.push(err5);
    }
    errors++;
  }
  var _valid0 = _errs1 === errors;
  if (_valid0) {
    valid0 = true;
    passing0 = 0;
    var props0 = {};
    props0.kind = true;
    props0.value = true;
  }
  const _errs7 = errors;
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.kind === void 0) {
      const err6 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
      if (vErrors === null) {
        vErrors = [err6];
      } else {
        vErrors.push(err6);
      }
      errors++;
    }
    if (data.value === void 0) {
      const err7 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "value" }, message: "must have required property 'value'" };
      if (vErrors === null) {
        vErrors = [err7];
      } else {
        vErrors.push(err7);
      }
      errors++;
    }
    if (data.kind !== void 0) {
      let data2 = data.kind;
      if (typeof data2 !== "string") {
        const err8 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/1/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err8];
        } else {
          vErrors.push(err8);
        }
        errors++;
      }
      if ("index" !== data2) {
        const err9 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/1/properties/kind/const", keyword: "const", params: { allowedValue: "index" }, message: "must be equal to constant" };
        if (vErrors === null) {
          vErrors = [err9];
        } else {
          vErrors.push(err9);
        }
        errors++;
      }
    }
    if (data.value !== void 0) {
      let data3 = data.value;
      if (!(typeof data3 == "number" && (!(data3 % 1) && !isNaN(data3)))) {
        const err10 = { instancePath: instancePath + "/value", schemaPath: "#/oneOf/1/properties/value/type", keyword: "type", params: { type: "integer" }, message: "must be integer" };
        if (vErrors === null) {
          vErrors = [err10];
        } else {
          vErrors.push(err10);
        }
        errors++;
      }
      if (typeof data3 == "number") {
        if (data3 < 0 || isNaN(data3)) {
          const err11 = { instancePath: instancePath + "/value", schemaPath: "#/oneOf/1/properties/value/minimum", keyword: "minimum", params: { comparison: ">=", limit: 0 }, message: "must be >= 0" };
          if (vErrors === null) {
            vErrors = [err11];
          } else {
            vErrors.push(err11);
          }
          errors++;
        }
        if (!formats4.validate(data3)) {
          const err12 = { instancePath: instancePath + "/value", schemaPath: "#/oneOf/1/properties/value/format", keyword: "format", params: { format: "uint32" }, message: 'must match format "uint32"' };
          if (vErrors === null) {
            vErrors = [err12];
          } else {
            vErrors.push(err12);
          }
          errors++;
        }
      }
    }
  } else {
    const err13 = { instancePath, schemaPath: "#/oneOf/1/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err13];
    } else {
      vErrors.push(err13);
    }
    errors++;
  }
  var _valid0 = _errs7 === errors;
  if (_valid0 && valid0) {
    valid0 = false;
    passing0 = [passing0, 1];
  } else {
    if (_valid0) {
      valid0 = true;
      passing0 = 1;
      if (props0 !== true) {
        props0 = props0 || {};
        props0.kind = true;
        props0.value = true;
      }
    }
  }
  if (!valid0) {
    const err14 = { instancePath, schemaPath: "#/oneOf", keyword: "oneOf", params: { passingSchemas: passing0 }, message: "must match exactly one schema in oneOf" };
    if (vErrors === null) {
      vErrors = [err14];
    } else {
      vErrors.push(err14);
    }
    errors++;
  } else {
    errors = _errs0;
    if (vErrors !== null) {
      if (_errs0) {
        vErrors.length = _errs0;
      } else {
        vErrors = null;
      }
    }
  }
  validate127.errors = vErrors;
  evaluated0.props = props0;
  return errors === 0;
}
validate127.evaluated = { "dynamicProps": true, "dynamicItems": false };
function validate126(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate126.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.sourceLabel === void 0) {
      const err0 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "sourceLabel" }, message: "must have required property 'sourceLabel'" };
      if (vErrors === null) {
        vErrors = [err0];
      } else {
        vErrors.push(err0);
      }
      errors++;
    }
    if (data.path === void 0) {
      const err1 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "path" }, message: "must have required property 'path'" };
      if (vErrors === null) {
        vErrors = [err1];
      } else {
        vErrors.push(err1);
      }
      errors++;
    }
    for (const key0 in data) {
      if (!(key0 === "path" || key0 === "sourceLabel")) {
        const err2 = { instancePath, schemaPath: "#/additionalProperties", keyword: "additionalProperties", params: { additionalProperty: key0 }, message: "must NOT have additional properties" };
        if (vErrors === null) {
          vErrors = [err2];
        } else {
          vErrors.push(err2);
        }
        errors++;
      }
    }
    if (data.path !== void 0) {
      let data0 = data.path;
      if (Array.isArray(data0)) {
        const len0 = data0.length;
        for (let i0 = 0; i0 < len0; i0++) {
          if (!validate127(data0[i0], { instancePath: instancePath + "/path/" + i0, parentData: data0, parentDataProperty: i0, rootData, dynamicAnchors })) {
            vErrors = vErrors === null ? validate127.errors : vErrors.concat(validate127.errors);
            errors = vErrors.length;
          }
        }
      } else {
        const err3 = { instancePath: instancePath + "/path", schemaPath: "#/properties/path/type", keyword: "type", params: { type: "array" }, message: "must be array" };
        if (vErrors === null) {
          vErrors = [err3];
        } else {
          vErrors.push(err3);
        }
        errors++;
      }
    }
    if (data.sourceLabel !== void 0) {
      if (typeof data.sourceLabel !== "string") {
        const err4 = { instancePath: instancePath + "/sourceLabel", schemaPath: "#/properties/sourceLabel/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err4];
        } else {
          vErrors.push(err4);
        }
        errors++;
      }
    }
  } else {
    const err5 = { instancePath, schemaPath: "#/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err5];
    } else {
      vErrors.push(err5);
    }
    errors++;
  }
  validate126.errors = vErrors;
  return errors === 0;
}
validate126.evaluated = { "props": true, "dynamicProps": false, "dynamicItems": false };
function validate130(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate130.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  const _errs0 = errors;
  let valid0 = false;
  let passing0 = null;
  const _errs1 = errors;
  if (typeof data !== "string") {
    const err0 = { instancePath, schemaPath: "#/oneOf/0/type", keyword: "type", params: { type: "string" }, message: "must be string" };
    if (vErrors === null) {
      vErrors = [err0];
    } else {
      vErrors.push(err0);
    }
    errors++;
  }
  if ("definition" !== data) {
    const err1 = { instancePath, schemaPath: "#/oneOf/0/const", keyword: "const", params: { allowedValue: "definition" }, message: "must be equal to constant" };
    if (vErrors === null) {
      vErrors = [err1];
    } else {
      vErrors.push(err1);
    }
    errors++;
  }
  var _valid0 = _errs1 === errors;
  if (_valid0) {
    valid0 = true;
    passing0 = 0;
  }
  const _errs3 = errors;
  if (typeof data !== "string") {
    const err2 = { instancePath, schemaPath: "#/oneOf/1/type", keyword: "type", params: { type: "string" }, message: "must be string" };
    if (vErrors === null) {
      vErrors = [err2];
    } else {
      vErrors.push(err2);
    }
    errors++;
  }
  if ("reference" !== data) {
    const err3 = { instancePath, schemaPath: "#/oneOf/1/const", keyword: "const", params: { allowedValue: "reference" }, message: "must be equal to constant" };
    if (vErrors === null) {
      vErrors = [err3];
    } else {
      vErrors.push(err3);
    }
    errors++;
  }
  var _valid0 = _errs3 === errors;
  if (_valid0 && valid0) {
    valid0 = false;
    passing0 = [passing0, 1];
  } else {
    if (_valid0) {
      valid0 = true;
      passing0 = 1;
    }
    const _errs5 = errors;
    if (typeof data !== "string") {
      const err4 = { instancePath, schemaPath: "#/oneOf/2/type", keyword: "type", params: { type: "string" }, message: "must be string" };
      if (vErrors === null) {
        vErrors = [err4];
      } else {
        vErrors.push(err4);
      }
      errors++;
    }
    if ("projection" !== data) {
      const err5 = { instancePath, schemaPath: "#/oneOf/2/const", keyword: "const", params: { allowedValue: "projection" }, message: "must be equal to constant" };
      if (vErrors === null) {
        vErrors = [err5];
      } else {
        vErrors.push(err5);
      }
      errors++;
    }
    var _valid0 = _errs5 === errors;
    if (_valid0 && valid0) {
      valid0 = false;
      passing0 = [passing0, 2];
    } else {
      if (_valid0) {
        valid0 = true;
        passing0 = 2;
      }
    }
  }
  if (!valid0) {
    const err6 = { instancePath, schemaPath: "#/oneOf", keyword: "oneOf", params: { passingSchemas: passing0 }, message: "must match exactly one schema in oneOf" };
    if (vErrors === null) {
      vErrors = [err6];
    } else {
      vErrors.push(err6);
    }
    errors++;
  } else {
    errors = _errs0;
    if (vErrors !== null) {
      if (_errs0) {
        vErrors.length = _errs0;
      } else {
        vErrors = null;
      }
    }
  }
  validate130.errors = vErrors;
  return errors === 0;
}
validate130.evaluated = { "dynamicProps": false, "dynamicItems": false };
function validate125(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate125.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.location === void 0) {
      const err0 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "location" }, message: "must have required property 'location'" };
      if (vErrors === null) {
        vErrors = [err0];
      } else {
        vErrors.push(err0);
      }
      errors++;
    }
    if (data.role === void 0) {
      const err1 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "role" }, message: "must have required property 'role'" };
      if (vErrors === null) {
        vErrors = [err1];
      } else {
        vErrors.push(err1);
      }
      errors++;
    }
    for (const key0 in data) {
      if (!(key0 === "location" || key0 === "role")) {
        const err2 = { instancePath, schemaPath: "#/additionalProperties", keyword: "additionalProperties", params: { additionalProperty: key0 }, message: "must NOT have additional properties" };
        if (vErrors === null) {
          vErrors = [err2];
        } else {
          vErrors.push(err2);
        }
        errors++;
      }
    }
    if (data.location !== void 0) {
      if (!validate126(data.location, { instancePath: instancePath + "/location", parentData: data, parentDataProperty: "location", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate126.errors : vErrors.concat(validate126.errors);
        errors = vErrors.length;
      }
    }
    if (data.role !== void 0) {
      if (!validate130(data.role, { instancePath: instancePath + "/role", parentData: data, parentDataProperty: "role", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate130.errors : vErrors.concat(validate130.errors);
        errors = vErrors.length;
      }
    }
  } else {
    const err3 = { instancePath, schemaPath: "#/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err3];
    } else {
      vErrors.push(err3);
    }
    errors++;
  }
  validate125.errors = vErrors;
  return errors === 0;
}
validate125.evaluated = { "props": true, "dynamicProps": false, "dynamicItems": false };
function validate124(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate124.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.subject === void 0) {
      const err0 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "subject" }, message: "must have required property 'subject'" };
      if (vErrors === null) {
        vErrors = [err0];
      } else {
        vErrors.push(err0);
      }
      errors++;
    }
    if (data.occurrences === void 0) {
      const err1 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "occurrences" }, message: "must have required property 'occurrences'" };
      if (vErrors === null) {
        vErrors = [err1];
      } else {
        vErrors.push(err1);
      }
      errors++;
    }
    if (data.inspected === void 0) {
      const err2 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "inspected" }, message: "must have required property 'inspected'" };
      if (vErrors === null) {
        vErrors = [err2];
      } else {
        vErrors.push(err2);
      }
      errors++;
    }
    if (data.sourcesComplete === void 0) {
      const err3 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "sourcesComplete" }, message: "must have required property 'sourcesComplete'" };
      if (vErrors === null) {
        vErrors = [err3];
      } else {
        vErrors.push(err3);
      }
      errors++;
    }
    for (const key0 in data) {
      if (!(key0 === "inspected" || key0 === "occurrences" || key0 === "recordAnchor" || key0 === "sourcesComplete" || key0 === "subject")) {
        const err4 = { instancePath, schemaPath: "#/additionalProperties", keyword: "additionalProperties", params: { additionalProperty: key0 }, message: "must NOT have additional properties" };
        if (vErrors === null) {
          vErrors = [err4];
        } else {
          vErrors.push(err4);
        }
        errors++;
      }
    }
    if (data.inspected !== void 0) {
      let data0 = data.inspected;
      if (!(typeof data0 == "number" && (!(data0 % 1) && !isNaN(data0)))) {
        const err5 = { instancePath: instancePath + "/inspected", schemaPath: "#/properties/inspected/type", keyword: "type", params: { type: "integer" }, message: "must be integer" };
        if (vErrors === null) {
          vErrors = [err5];
        } else {
          vErrors.push(err5);
        }
        errors++;
      }
      if (typeof data0 == "number") {
        if (data0 > 65535 || isNaN(data0)) {
          const err6 = { instancePath: instancePath + "/inspected", schemaPath: "#/properties/inspected/maximum", keyword: "maximum", params: { comparison: "<=", limit: 65535 }, message: "must be <= 65535" };
          if (vErrors === null) {
            vErrors = [err6];
          } else {
            vErrors.push(err6);
          }
          errors++;
        }
        if (data0 < 0 || isNaN(data0)) {
          const err7 = { instancePath: instancePath + "/inspected", schemaPath: "#/properties/inspected/minimum", keyword: "minimum", params: { comparison: ">=", limit: 0 }, message: "must be >= 0" };
          if (vErrors === null) {
            vErrors = [err7];
          } else {
            vErrors.push(err7);
          }
          errors++;
        }
        if (!formats42.validate(data0)) {
          const err8 = { instancePath: instancePath + "/inspected", schemaPath: "#/properties/inspected/format", keyword: "format", params: { format: "uint16" }, message: 'must match format "uint16"' };
          if (vErrors === null) {
            vErrors = [err8];
          } else {
            vErrors.push(err8);
          }
          errors++;
        }
      }
    }
    if (data.occurrences !== void 0) {
      let data1 = data.occurrences;
      if (Array.isArray(data1)) {
        const len0 = data1.length;
        for (let i0 = 0; i0 < len0; i0++) {
          if (!validate125(data1[i0], { instancePath: instancePath + "/occurrences/" + i0, parentData: data1, parentDataProperty: i0, rootData, dynamicAnchors })) {
            vErrors = vErrors === null ? validate125.errors : vErrors.concat(validate125.errors);
            errors = vErrors.length;
          }
        }
      } else {
        const err9 = { instancePath: instancePath + "/occurrences", schemaPath: "#/properties/occurrences/type", keyword: "type", params: { type: "array" }, message: "must be array" };
        if (vErrors === null) {
          vErrors = [err9];
        } else {
          vErrors.push(err9);
        }
        errors++;
      }
    }
    if (data.recordAnchor !== void 0) {
      if (!validate126(data.recordAnchor, { instancePath: instancePath + "/recordAnchor", parentData: data, parentDataProperty: "recordAnchor", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate126.errors : vErrors.concat(validate126.errors);
        errors = vErrors.length;
      }
    }
    if (data.sourcesComplete !== void 0) {
      if (typeof data.sourcesComplete !== "boolean") {
        const err10 = { instancePath: instancePath + "/sourcesComplete", schemaPath: "#/properties/sourcesComplete/type", keyword: "type", params: { type: "boolean" }, message: "must be boolean" };
        if (vErrors === null) {
          vErrors = [err10];
        } else {
          vErrors.push(err10);
        }
        errors++;
      }
    }
    if (data.subject !== void 0) {
      if (!validate68(data.subject, { instancePath: instancePath + "/subject", parentData: data, parentDataProperty: "subject", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate68.errors : vErrors.concat(validate68.errors);
        errors = vErrors.length;
      }
    }
  } else {
    const err11 = { instancePath, schemaPath: "#/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err11];
    } else {
      vErrors.push(err11);
    }
    errors++;
  }
  validate124.errors = vErrors;
  return errors === 0;
}
validate124.evaluated = { "props": true, "dynamicProps": false, "dynamicItems": false };
function validate138(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate138.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  const _errs0 = errors;
  let valid0 = false;
  let passing0 = null;
  const _errs1 = errors;
  if (typeof data !== "string") {
    const err0 = { instancePath, schemaPath: "#/oneOf/0/type", keyword: "type", params: { type: "string" }, message: "must be string" };
    if (vErrors === null) {
      vErrors = [err0];
    } else {
      vErrors.push(err0);
    }
    errors++;
  }
  if ("accepted" !== data) {
    const err1 = { instancePath, schemaPath: "#/oneOf/0/const", keyword: "const", params: { allowedValue: "accepted" }, message: "must be equal to constant" };
    if (vErrors === null) {
      vErrors = [err1];
    } else {
      vErrors.push(err1);
    }
    errors++;
  }
  var _valid0 = _errs1 === errors;
  if (_valid0) {
    valid0 = true;
    passing0 = 0;
  }
  const _errs3 = errors;
  if (typeof data !== "string") {
    const err2 = { instancePath, schemaPath: "#/oneOf/1/type", keyword: "type", params: { type: "string" }, message: "must be string" };
    if (vErrors === null) {
      vErrors = [err2];
    } else {
      vErrors.push(err2);
    }
    errors++;
  }
  if ("rejected" !== data) {
    const err3 = { instancePath, schemaPath: "#/oneOf/1/const", keyword: "const", params: { allowedValue: "rejected" }, message: "must be equal to constant" };
    if (vErrors === null) {
      vErrors = [err3];
    } else {
      vErrors.push(err3);
    }
    errors++;
  }
  var _valid0 = _errs3 === errors;
  if (_valid0 && valid0) {
    valid0 = false;
    passing0 = [passing0, 1];
  } else {
    if (_valid0) {
      valid0 = true;
      passing0 = 1;
    }
    const _errs5 = errors;
    if (typeof data !== "string") {
      const err4 = { instancePath, schemaPath: "#/oneOf/2/type", keyword: "type", params: { type: "string" }, message: "must be string" };
      if (vErrors === null) {
        vErrors = [err4];
      } else {
        vErrors.push(err4);
      }
      errors++;
    }
    if ("cancelled" !== data) {
      const err5 = { instancePath, schemaPath: "#/oneOf/2/const", keyword: "const", params: { allowedValue: "cancelled" }, message: "must be equal to constant" };
      if (vErrors === null) {
        vErrors = [err5];
      } else {
        vErrors.push(err5);
      }
      errors++;
    }
    var _valid0 = _errs5 === errors;
    if (_valid0 && valid0) {
      valid0 = false;
      passing0 = [passing0, 2];
    } else {
      if (_valid0) {
        valid0 = true;
        passing0 = 2;
      }
      const _errs7 = errors;
      if (typeof data !== "string") {
        const err6 = { instancePath, schemaPath: "#/oneOf/3/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err6];
        } else {
          vErrors.push(err6);
        }
        errors++;
      }
      if ("stale" !== data) {
        const err7 = { instancePath, schemaPath: "#/oneOf/3/const", keyword: "const", params: { allowedValue: "stale" }, message: "must be equal to constant" };
        if (vErrors === null) {
          vErrors = [err7];
        } else {
          vErrors.push(err7);
        }
        errors++;
      }
      var _valid0 = _errs7 === errors;
      if (_valid0 && valid0) {
        valid0 = false;
        passing0 = [passing0, 3];
      } else {
        if (_valid0) {
          valid0 = true;
          passing0 = 3;
        }
      }
    }
  }
  if (!valid0) {
    const err8 = { instancePath, schemaPath: "#/oneOf", keyword: "oneOf", params: { passingSchemas: passing0 }, message: "must match exactly one schema in oneOf" };
    if (vErrors === null) {
      vErrors = [err8];
    } else {
      vErrors.push(err8);
    }
    errors++;
  } else {
    errors = _errs0;
    if (vErrors !== null) {
      if (_errs0) {
        vErrors.length = _errs0;
      } else {
        vErrors = null;
      }
    }
  }
  validate138.errors = vErrors;
  return errors === 0;
}
validate138.evaluated = { "dynamicProps": false, "dynamicItems": false };
function validate250(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate250.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  const _errs0 = errors;
  let valid0 = false;
  let passing0 = null;
  const _errs1 = errors;
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.state === void 0) {
      const err0 = { instancePath, schemaPath: "#/oneOf/0/required", keyword: "required", params: { missingProperty: "state" }, message: "must have required property 'state'" };
      if (vErrors === null) {
        vErrors = [err0];
      } else {
        vErrors.push(err0);
      }
      errors++;
    }
    if (data.context === void 0) {
      const err1 = { instancePath, schemaPath: "#/oneOf/0/required", keyword: "required", params: { missingProperty: "context" }, message: "must have required property 'context'" };
      if (vErrors === null) {
        vErrors = [err1];
      } else {
        vErrors.push(err1);
      }
      errors++;
    }
    if (data.baseline === void 0) {
      const err2 = { instancePath, schemaPath: "#/oneOf/0/required", keyword: "required", params: { missingProperty: "baseline" }, message: "must have required property 'baseline'" };
      if (vErrors === null) {
        vErrors = [err2];
      } else {
        vErrors.push(err2);
      }
      errors++;
    }
    for (const key0 in data) {
      if (!(key0 === "baseline" || key0 === "context" || key0 === "state")) {
        const err3 = { instancePath, schemaPath: "#/oneOf/0/additionalProperties", keyword: "additionalProperties", params: { additionalProperty: key0 }, message: "must NOT have additional properties" };
        if (vErrors === null) {
          vErrors = [err3];
        } else {
          vErrors.push(err3);
        }
        errors++;
      }
    }
    if (data.baseline !== void 0) {
      if (!validate77(data.baseline, { instancePath: instancePath + "/baseline", parentData: data, parentDataProperty: "baseline", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate77.errors : vErrors.concat(validate77.errors);
        errors = vErrors.length;
      }
    }
    if (data.context !== void 0) {
      if (!validate79(data.context, { instancePath: instancePath + "/context", parentData: data, parentDataProperty: "context", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate79.errors : vErrors.concat(validate79.errors);
        errors = vErrors.length;
      }
    }
    if (data.state !== void 0) {
      let data2 = data.state;
      if (typeof data2 !== "string") {
        const err4 = { instancePath: instancePath + "/state", schemaPath: "#/oneOf/0/properties/state/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err4];
        } else {
          vErrors.push(err4);
        }
        errors++;
      }
      if ("explained" !== data2) {
        const err5 = { instancePath: instancePath + "/state", schemaPath: "#/oneOf/0/properties/state/const", keyword: "const", params: { allowedValue: "explained" }, message: "must be equal to constant" };
        if (vErrors === null) {
          vErrors = [err5];
        } else {
          vErrors.push(err5);
        }
        errors++;
      }
    }
  } else {
    const err6 = { instancePath, schemaPath: "#/oneOf/0/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err6];
    } else {
      vErrors.push(err6);
    }
    errors++;
  }
  var _valid0 = _errs1 === errors;
  if (_valid0) {
    valid0 = true;
    passing0 = 0;
    var props0 = true;
  }
  const _errs8 = errors;
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.state === void 0) {
      const err7 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "state" }, message: "must have required property 'state'" };
      if (vErrors === null) {
        vErrors = [err7];
      } else {
        vErrors.push(err7);
      }
      errors++;
    }
    if (data.context === void 0) {
      const err8 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "context" }, message: "must have required property 'context'" };
      if (vErrors === null) {
        vErrors = [err8];
      } else {
        vErrors.push(err8);
      }
      errors++;
    }
    if (data.coordinate === void 0) {
      const err9 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "coordinate" }, message: "must have required property 'coordinate'" };
      if (vErrors === null) {
        vErrors = [err9];
      } else {
        vErrors.push(err9);
      }
      errors++;
    }
    if (data.preview === void 0) {
      const err10 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "preview" }, message: "must have required property 'preview'" };
      if (vErrors === null) {
        vErrors = [err10];
      } else {
        vErrors.push(err10);
      }
      errors++;
    }
    for (const key1 in data) {
      if (!(key1 === "context" || key1 === "coordinate" || key1 === "preview" || key1 === "state")) {
        const err11 = { instancePath, schemaPath: "#/oneOf/1/additionalProperties", keyword: "additionalProperties", params: { additionalProperty: key1 }, message: "must NOT have additional properties" };
        if (vErrors === null) {
          vErrors = [err11];
        } else {
          vErrors.push(err11);
        }
        errors++;
      }
    }
    if (data.context !== void 0) {
      if (!validate79(data.context, { instancePath: instancePath + "/context", parentData: data, parentDataProperty: "context", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate79.errors : vErrors.concat(validate79.errors);
        errors = vErrors.length;
      }
    }
    if (data.coordinate !== void 0) {
      if (!validate82(data.coordinate, { instancePath: instancePath + "/coordinate", parentData: data, parentDataProperty: "coordinate", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate82.errors : vErrors.concat(validate82.errors);
        errors = vErrors.length;
      }
    }
    if (data.preview !== void 0) {
      if (!validate86(data.preview, { instancePath: instancePath + "/preview", parentData: data, parentDataProperty: "preview", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate86.errors : vErrors.concat(validate86.errors);
        errors = vErrors.length;
      }
    }
    if (data.state !== void 0) {
      let data6 = data.state;
      if (typeof data6 !== "string") {
        const err12 = { instancePath: instancePath + "/state", schemaPath: "#/oneOf/1/properties/state/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err12];
        } else {
          vErrors.push(err12);
        }
        errors++;
      }
      if ("previewed" !== data6) {
        const err13 = { instancePath: instancePath + "/state", schemaPath: "#/oneOf/1/properties/state/const", keyword: "const", params: { allowedValue: "previewed" }, message: "must be equal to constant" };
        if (vErrors === null) {
          vErrors = [err13];
        } else {
          vErrors.push(err13);
        }
        errors++;
      }
    }
  } else {
    const err14 = { instancePath, schemaPath: "#/oneOf/1/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err14];
    } else {
      vErrors.push(err14);
    }
    errors++;
  }
  var _valid0 = _errs8 === errors;
  if (_valid0 && valid0) {
    valid0 = false;
    passing0 = [passing0, 1];
  } else {
    if (_valid0) {
      valid0 = true;
      passing0 = 1;
      if (props0 !== true) {
        props0 = true;
      }
    }
    const _errs16 = errors;
    if (data && typeof data == "object" && !Array.isArray(data)) {
      if (data.state === void 0) {
        const err15 = { instancePath, schemaPath: "#/oneOf/2/required", keyword: "required", params: { missingProperty: "state" }, message: "must have required property 'state'" };
        if (vErrors === null) {
          vErrors = [err15];
        } else {
          vErrors.push(err15);
        }
        errors++;
      }
      if (data.context === void 0) {
        const err16 = { instancePath, schemaPath: "#/oneOf/2/required", keyword: "required", params: { missingProperty: "context" }, message: "must have required property 'context'" };
        if (vErrors === null) {
          vErrors = [err16];
        } else {
          vErrors.push(err16);
        }
        errors++;
      }
      if (data.coordinate === void 0) {
        const err17 = { instancePath, schemaPath: "#/oneOf/2/required", keyword: "required", params: { missingProperty: "coordinate" }, message: "must have required property 'coordinate'" };
        if (vErrors === null) {
          vErrors = [err17];
        } else {
          vErrors.push(err17);
        }
        errors++;
      }
      if (data.changeRoots === void 0) {
        const err18 = { instancePath, schemaPath: "#/oneOf/2/required", keyword: "required", params: { missingProperty: "changeRoots" }, message: "must have required property 'changeRoots'" };
        if (vErrors === null) {
          vErrors = [err18];
        } else {
          vErrors.push(err18);
        }
        errors++;
      }
      if (data.impacts === void 0) {
        const err19 = { instancePath, schemaPath: "#/oneOf/2/required", keyword: "required", params: { missingProperty: "impacts" }, message: "must have required property 'impacts'" };
        if (vErrors === null) {
          vErrors = [err19];
        } else {
          vErrors.push(err19);
        }
        errors++;
      }
      for (const key2 in data) {
        if (!(key2 === "changeRoots" || key2 === "context" || key2 === "coordinate" || key2 === "impacts" || key2 === "state")) {
          const err20 = { instancePath, schemaPath: "#/oneOf/2/additionalProperties", keyword: "additionalProperties", params: { additionalProperty: key2 }, message: "must NOT have additional properties" };
          if (vErrors === null) {
            vErrors = [err20];
          } else {
            vErrors.push(err20);
          }
          errors++;
        }
      }
      if (data.changeRoots !== void 0) {
        if (!validate59(data.changeRoots, { instancePath: instancePath + "/changeRoots", parentData: data, parentDataProperty: "changeRoots", rootData, dynamicAnchors })) {
          vErrors = vErrors === null ? validate59.errors : vErrors.concat(validate59.errors);
          errors = vErrors.length;
        }
      }
      if (data.context !== void 0) {
        if (!validate79(data.context, { instancePath: instancePath + "/context", parentData: data, parentDataProperty: "context", rootData, dynamicAnchors })) {
          vErrors = vErrors === null ? validate79.errors : vErrors.concat(validate79.errors);
          errors = vErrors.length;
        }
      }
      if (data.coordinate !== void 0) {
        if (!validate82(data.coordinate, { instancePath: instancePath + "/coordinate", parentData: data, parentDataProperty: "coordinate", rootData, dynamicAnchors })) {
          vErrors = vErrors === null ? validate82.errors : vErrors.concat(validate82.errors);
          errors = vErrors.length;
        }
      }
      if (data.impacts !== void 0) {
        if (!validate97(data.impacts, { instancePath: instancePath + "/impacts", parentData: data, parentDataProperty: "impacts", rootData, dynamicAnchors })) {
          vErrors = vErrors === null ? validate97.errors : vErrors.concat(validate97.errors);
          errors = vErrors.length;
        }
      }
      if (data.state !== void 0) {
        let data11 = data.state;
        if (typeof data11 !== "string") {
          const err21 = { instancePath: instancePath + "/state", schemaPath: "#/oneOf/2/properties/state/type", keyword: "type", params: { type: "string" }, message: "must be string" };
          if (vErrors === null) {
            vErrors = [err21];
          } else {
            vErrors.push(err21);
          }
          errors++;
        }
        if ("details" !== data11) {
          const err22 = { instancePath: instancePath + "/state", schemaPath: "#/oneOf/2/properties/state/const", keyword: "const", params: { allowedValue: "details" }, message: "must be equal to constant" };
          if (vErrors === null) {
            vErrors = [err22];
          } else {
            vErrors.push(err22);
          }
          errors++;
        }
      }
    } else {
      const err23 = { instancePath, schemaPath: "#/oneOf/2/type", keyword: "type", params: { type: "object" }, message: "must be object" };
      if (vErrors === null) {
        vErrors = [err23];
      } else {
        vErrors.push(err23);
      }
      errors++;
    }
    var _valid0 = _errs16 === errors;
    if (_valid0 && valid0) {
      valid0 = false;
      passing0 = [passing0, 2];
    } else {
      if (_valid0) {
        valid0 = true;
        passing0 = 2;
        if (props0 !== true) {
          props0 = true;
        }
      }
      const _errs25 = errors;
      if (data && typeof data == "object" && !Array.isArray(data)) {
        if (data.state === void 0) {
          const err24 = { instancePath, schemaPath: "#/oneOf/3/required", keyword: "required", params: { missingProperty: "state" }, message: "must have required property 'state'" };
          if (vErrors === null) {
            vErrors = [err24];
          } else {
            vErrors.push(err24);
          }
          errors++;
        }
        if (data.context === void 0) {
          const err25 = { instancePath, schemaPath: "#/oneOf/3/required", keyword: "required", params: { missingProperty: "context" }, message: "must have required property 'context'" };
          if (vErrors === null) {
            vErrors = [err25];
          } else {
            vErrors.push(err25);
          }
          errors++;
        }
        if (data.coordinate === void 0) {
          const err26 = { instancePath, schemaPath: "#/oneOf/3/required", keyword: "required", params: { missingProperty: "coordinate" }, message: "must have required property 'coordinate'" };
          if (vErrors === null) {
            vErrors = [err26];
          } else {
            vErrors.push(err26);
          }
          errors++;
        }
        if (data.explanation === void 0) {
          const err27 = { instancePath, schemaPath: "#/oneOf/3/required", keyword: "required", params: { missingProperty: "explanation" }, message: "must have required property 'explanation'" };
          if (vErrors === null) {
            vErrors = [err27];
          } else {
            vErrors.push(err27);
          }
          errors++;
        }
        if (data.source === void 0) {
          const err28 = { instancePath, schemaPath: "#/oneOf/3/required", keyword: "required", params: { missingProperty: "source" }, message: "must have required property 'source'" };
          if (vErrors === null) {
            vErrors = [err28];
          } else {
            vErrors.push(err28);
          }
          errors++;
        }
        for (const key3 in data) {
          if (!(key3 === "context" || key3 === "coordinate" || key3 === "explanation" || key3 === "source" || key3 === "state")) {
            const err29 = { instancePath, schemaPath: "#/oneOf/3/additionalProperties", keyword: "additionalProperties", params: { additionalProperty: key3 }, message: "must NOT have additional properties" };
            if (vErrors === null) {
              vErrors = [err29];
            } else {
              vErrors.push(err29);
            }
            errors++;
          }
        }
        if (data.context !== void 0) {
          if (!validate79(data.context, { instancePath: instancePath + "/context", parentData: data, parentDataProperty: "context", rootData, dynamicAnchors })) {
            vErrors = vErrors === null ? validate79.errors : vErrors.concat(validate79.errors);
            errors = vErrors.length;
          }
        }
        if (data.coordinate !== void 0) {
          if (!validate82(data.coordinate, { instancePath: instancePath + "/coordinate", parentData: data, parentDataProperty: "coordinate", rootData, dynamicAnchors })) {
            vErrors = vErrors === null ? validate82.errors : vErrors.concat(validate82.errors);
            errors = vErrors.length;
          }
        }
        if (data.explanation !== void 0) {
          if (!validate106(data.explanation, { instancePath: instancePath + "/explanation", parentData: data, parentDataProperty: "explanation", rootData, dynamicAnchors })) {
            vErrors = vErrors === null ? validate106.errors : vErrors.concat(validate106.errors);
            errors = vErrors.length;
          }
        }
        if (data.source !== void 0) {
          if (!validate124(data.source, { instancePath: instancePath + "/source", parentData: data, parentDataProperty: "source", rootData, dynamicAnchors })) {
            vErrors = vErrors === null ? validate124.errors : vErrors.concat(validate124.errors);
            errors = vErrors.length;
          }
        }
        if (data.state !== void 0) {
          let data16 = data.state;
          if (typeof data16 !== "string") {
            const err30 = { instancePath: instancePath + "/state", schemaPath: "#/oneOf/3/properties/state/type", keyword: "type", params: { type: "string" }, message: "must be string" };
            if (vErrors === null) {
              vErrors = [err30];
            } else {
              vErrors.push(err30);
            }
            errors++;
          }
          if ("subject-details" !== data16) {
            const err31 = { instancePath: instancePath + "/state", schemaPath: "#/oneOf/3/properties/state/const", keyword: "const", params: { allowedValue: "subject-details" }, message: "must be equal to constant" };
            if (vErrors === null) {
              vErrors = [err31];
            } else {
              vErrors.push(err31);
            }
            errors++;
          }
        }
      } else {
        const err32 = { instancePath, schemaPath: "#/oneOf/3/type", keyword: "type", params: { type: "object" }, message: "must be object" };
        if (vErrors === null) {
          vErrors = [err32];
        } else {
          vErrors.push(err32);
        }
        errors++;
      }
      var _valid0 = _errs25 === errors;
      if (_valid0 && valid0) {
        valid0 = false;
        passing0 = [passing0, 3];
      } else {
        if (_valid0) {
          valid0 = true;
          passing0 = 3;
          if (props0 !== true) {
            props0 = true;
          }
        }
        const _errs34 = errors;
        if (data && typeof data == "object" && !Array.isArray(data)) {
          if (data.state === void 0) {
            const err33 = { instancePath, schemaPath: "#/oneOf/4/required", keyword: "required", params: { missingProperty: "state" }, message: "must have required property 'state'" };
            if (vErrors === null) {
              vErrors = [err33];
            } else {
              vErrors.push(err33);
            }
            errors++;
          }
          if (data.context === void 0) {
            const err34 = { instancePath, schemaPath: "#/oneOf/4/required", keyword: "required", params: { missingProperty: "context" }, message: "must have required property 'context'" };
            if (vErrors === null) {
              vErrors = [err34];
            } else {
              vErrors.push(err34);
            }
            errors++;
          }
          if (data.disposition === void 0) {
            const err35 = { instancePath, schemaPath: "#/oneOf/4/required", keyword: "required", params: { missingProperty: "disposition" }, message: "must have required property 'disposition'" };
            if (vErrors === null) {
              vErrors = [err35];
            } else {
              vErrors.push(err35);
            }
            errors++;
          }
          if (data.coordinate === void 0) {
            const err36 = { instancePath, schemaPath: "#/oneOf/4/required", keyword: "required", params: { missingProperty: "coordinate" }, message: "must have required property 'coordinate'" };
            if (vErrors === null) {
              vErrors = [err36];
            } else {
              vErrors.push(err36);
            }
            errors++;
          }
          if (data.preview === void 0) {
            const err37 = { instancePath, schemaPath: "#/oneOf/4/required", keyword: "required", params: { missingProperty: "preview" }, message: "must have required property 'preview'" };
            if (vErrors === null) {
              vErrors = [err37];
            } else {
              vErrors.push(err37);
            }
            errors++;
          }
          for (const key4 in data) {
            if (!(key4 === "context" || key4 === "coordinate" || key4 === "disposition" || key4 === "preview" || key4 === "state")) {
              const err38 = { instancePath, schemaPath: "#/oneOf/4/additionalProperties", keyword: "additionalProperties", params: { additionalProperty: key4 }, message: "must NOT have additional properties" };
              if (vErrors === null) {
                vErrors = [err38];
              } else {
                vErrors.push(err38);
              }
              errors++;
            }
          }
          if (data.context !== void 0) {
            if (!validate79(data.context, { instancePath: instancePath + "/context", parentData: data, parentDataProperty: "context", rootData, dynamicAnchors })) {
              vErrors = vErrors === null ? validate79.errors : vErrors.concat(validate79.errors);
              errors = vErrors.length;
            }
          }
          if (data.coordinate !== void 0) {
            if (!validate82(data.coordinate, { instancePath: instancePath + "/coordinate", parentData: data, parentDataProperty: "coordinate", rootData, dynamicAnchors })) {
              vErrors = vErrors === null ? validate82.errors : vErrors.concat(validate82.errors);
              errors = vErrors.length;
            }
          }
          if (data.disposition !== void 0) {
            if (!validate138(data.disposition, { instancePath: instancePath + "/disposition", parentData: data, parentDataProperty: "disposition", rootData, dynamicAnchors })) {
              vErrors = vErrors === null ? validate138.errors : vErrors.concat(validate138.errors);
              errors = vErrors.length;
            }
          }
          if (data.preview !== void 0) {
            if (!validate86(data.preview, { instancePath: instancePath + "/preview", parentData: data, parentDataProperty: "preview", rootData, dynamicAnchors })) {
              vErrors = vErrors === null ? validate86.errors : vErrors.concat(validate86.errors);
              errors = vErrors.length;
            }
          }
          if (data.state !== void 0) {
            let data21 = data.state;
            if (typeof data21 !== "string") {
              const err39 = { instancePath: instancePath + "/state", schemaPath: "#/oneOf/4/properties/state/type", keyword: "type", params: { type: "string" }, message: "must be string" };
              if (vErrors === null) {
                vErrors = [err39];
              } else {
                vErrors.push(err39);
              }
              errors++;
            }
            if ("terminal" !== data21) {
              const err40 = { instancePath: instancePath + "/state", schemaPath: "#/oneOf/4/properties/state/const", keyword: "const", params: { allowedValue: "terminal" }, message: "must be equal to constant" };
              if (vErrors === null) {
                vErrors = [err40];
              } else {
                vErrors.push(err40);
              }
              errors++;
            }
          }
        } else {
          const err41 = { instancePath, schemaPath: "#/oneOf/4/type", keyword: "type", params: { type: "object" }, message: "must be object" };
          if (vErrors === null) {
            vErrors = [err41];
          } else {
            vErrors.push(err41);
          }
          errors++;
        }
        var _valid0 = _errs34 === errors;
        if (_valid0 && valid0) {
          valid0 = false;
          passing0 = [passing0, 4];
        } else {
          if (_valid0) {
            valid0 = true;
            passing0 = 4;
            if (props0 !== true) {
              props0 = true;
            }
          }
        }
      }
    }
  }
  if (!valid0) {
    const err42 = { instancePath, schemaPath: "#/oneOf", keyword: "oneOf", params: { passingSchemas: passing0 }, message: "must match exactly one schema in oneOf" };
    if (vErrors === null) {
      vErrors = [err42];
    } else {
      vErrors.push(err42);
    }
    errors++;
  } else {
    errors = _errs0;
    if (vErrors !== null) {
      if (_errs0) {
        vErrors.length = _errs0;
      } else {
        vErrors = null;
      }
    }
  }
  validate250.errors = vErrors;
  evaluated0.props = props0;
  return errors === 0;
}
validate250.evaluated = { "dynamicProps": true, "dynamicItems": false };
var validateEffectiveProfileExplanationResponse = validate268;
var schema53 = { "additionalProperties": false, "description": "Exact semantic coordinates shared by every page of one profile inspection.", "properties": { "compositionPlanDigest": { "description": "Identity of the neutral experience-composition proof.", "type": "string" }, "executionWorkspaceId": { "description": "Selected logical execution workspace.", "type": "string" }, "instanceDigest": { "description": "Canonical private machine-resolved instance identity.\n\nThis coordinate proves which admitted private projection was compiled;\nit does not claim machine or native realization.", "type": "string" }, "machineId": { "description": "Selected private machine-binding identity.", "type": "string" }, "packageSemanticIrDigest": { "description": "Identity of the admitted Package semantic closure.", "type": "string" }, "profileDigest": { "description": "Canonical portable profile identity.", "type": "string" }, "projectId": { "description": "Selected logical project.", "type": "string" }, "selectedTopologyDigest": { "description": "Identity of the exact project-relative desired topology selection.", "type": "string" }, "workspaceAdmissionEvidenceDigest": { "description": "Identity of the owner-proof set used for workspace admission.", "type": "string" }, "workspaceId": { "description": "Principal workspace owning the desired graph.", "type": "string" }, "workspaceSemanticIrDigest": { "description": "Identity of the admitted workspace semantic graph.", "type": "string" } }, "required": ["workspaceId", "projectId", "executionWorkspaceId", "machineId", "profileDigest", "instanceDigest", "selectedTopologyDigest", "packageSemanticIrDigest", "compositionPlanDigest", "workspaceSemanticIrDigest", "workspaceAdmissionEvidenceDigest"], "type": "object" };
function validate144(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate144.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.workspaceId === void 0) {
      const err0 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "workspaceId" }, message: "must have required property 'workspaceId'" };
      if (vErrors === null) {
        vErrors = [err0];
      } else {
        vErrors.push(err0);
      }
      errors++;
    }
    if (data.projectId === void 0) {
      const err1 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "projectId" }, message: "must have required property 'projectId'" };
      if (vErrors === null) {
        vErrors = [err1];
      } else {
        vErrors.push(err1);
      }
      errors++;
    }
    if (data.executionWorkspaceId === void 0) {
      const err2 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "executionWorkspaceId" }, message: "must have required property 'executionWorkspaceId'" };
      if (vErrors === null) {
        vErrors = [err2];
      } else {
        vErrors.push(err2);
      }
      errors++;
    }
    if (data.machineId === void 0) {
      const err3 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "machineId" }, message: "must have required property 'machineId'" };
      if (vErrors === null) {
        vErrors = [err3];
      } else {
        vErrors.push(err3);
      }
      errors++;
    }
    if (data.profileDigest === void 0) {
      const err4 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "profileDigest" }, message: "must have required property 'profileDigest'" };
      if (vErrors === null) {
        vErrors = [err4];
      } else {
        vErrors.push(err4);
      }
      errors++;
    }
    if (data.instanceDigest === void 0) {
      const err5 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "instanceDigest" }, message: "must have required property 'instanceDigest'" };
      if (vErrors === null) {
        vErrors = [err5];
      } else {
        vErrors.push(err5);
      }
      errors++;
    }
    if (data.selectedTopologyDigest === void 0) {
      const err6 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "selectedTopologyDigest" }, message: "must have required property 'selectedTopologyDigest'" };
      if (vErrors === null) {
        vErrors = [err6];
      } else {
        vErrors.push(err6);
      }
      errors++;
    }
    if (data.packageSemanticIrDigest === void 0) {
      const err7 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "packageSemanticIrDigest" }, message: "must have required property 'packageSemanticIrDigest'" };
      if (vErrors === null) {
        vErrors = [err7];
      } else {
        vErrors.push(err7);
      }
      errors++;
    }
    if (data.compositionPlanDigest === void 0) {
      const err8 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "compositionPlanDigest" }, message: "must have required property 'compositionPlanDigest'" };
      if (vErrors === null) {
        vErrors = [err8];
      } else {
        vErrors.push(err8);
      }
      errors++;
    }
    if (data.workspaceSemanticIrDigest === void 0) {
      const err9 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "workspaceSemanticIrDigest" }, message: "must have required property 'workspaceSemanticIrDigest'" };
      if (vErrors === null) {
        vErrors = [err9];
      } else {
        vErrors.push(err9);
      }
      errors++;
    }
    if (data.workspaceAdmissionEvidenceDigest === void 0) {
      const err10 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "workspaceAdmissionEvidenceDigest" }, message: "must have required property 'workspaceAdmissionEvidenceDigest'" };
      if (vErrors === null) {
        vErrors = [err10];
      } else {
        vErrors.push(err10);
      }
      errors++;
    }
    for (const key0 in data) {
      if (!func1.call(schema53.properties, key0)) {
        const err11 = { instancePath, schemaPath: "#/additionalProperties", keyword: "additionalProperties", params: { additionalProperty: key0 }, message: "must NOT have additional properties" };
        if (vErrors === null) {
          vErrors = [err11];
        } else {
          vErrors.push(err11);
        }
        errors++;
      }
    }
    if (data.compositionPlanDigest !== void 0) {
      if (typeof data.compositionPlanDigest !== "string") {
        const err12 = { instancePath: instancePath + "/compositionPlanDigest", schemaPath: "#/properties/compositionPlanDigest/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err12];
        } else {
          vErrors.push(err12);
        }
        errors++;
      }
    }
    if (data.executionWorkspaceId !== void 0) {
      if (typeof data.executionWorkspaceId !== "string") {
        const err13 = { instancePath: instancePath + "/executionWorkspaceId", schemaPath: "#/properties/executionWorkspaceId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err13];
        } else {
          vErrors.push(err13);
        }
        errors++;
      }
    }
    if (data.instanceDigest !== void 0) {
      if (typeof data.instanceDigest !== "string") {
        const err14 = { instancePath: instancePath + "/instanceDigest", schemaPath: "#/properties/instanceDigest/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err14];
        } else {
          vErrors.push(err14);
        }
        errors++;
      }
    }
    if (data.machineId !== void 0) {
      if (typeof data.machineId !== "string") {
        const err15 = { instancePath: instancePath + "/machineId", schemaPath: "#/properties/machineId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err15];
        } else {
          vErrors.push(err15);
        }
        errors++;
      }
    }
    if (data.packageSemanticIrDigest !== void 0) {
      if (typeof data.packageSemanticIrDigest !== "string") {
        const err16 = { instancePath: instancePath + "/packageSemanticIrDigest", schemaPath: "#/properties/packageSemanticIrDigest/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err16];
        } else {
          vErrors.push(err16);
        }
        errors++;
      }
    }
    if (data.profileDigest !== void 0) {
      if (typeof data.profileDigest !== "string") {
        const err17 = { instancePath: instancePath + "/profileDigest", schemaPath: "#/properties/profileDigest/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err17];
        } else {
          vErrors.push(err17);
        }
        errors++;
      }
    }
    if (data.projectId !== void 0) {
      if (typeof data.projectId !== "string") {
        const err18 = { instancePath: instancePath + "/projectId", schemaPath: "#/properties/projectId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err18];
        } else {
          vErrors.push(err18);
        }
        errors++;
      }
    }
    if (data.selectedTopologyDigest !== void 0) {
      if (typeof data.selectedTopologyDigest !== "string") {
        const err19 = { instancePath: instancePath + "/selectedTopologyDigest", schemaPath: "#/properties/selectedTopologyDigest/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err19];
        } else {
          vErrors.push(err19);
        }
        errors++;
      }
    }
    if (data.workspaceAdmissionEvidenceDigest !== void 0) {
      if (typeof data.workspaceAdmissionEvidenceDigest !== "string") {
        const err20 = { instancePath: instancePath + "/workspaceAdmissionEvidenceDigest", schemaPath: "#/properties/workspaceAdmissionEvidenceDigest/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err20];
        } else {
          vErrors.push(err20);
        }
        errors++;
      }
    }
    if (data.workspaceId !== void 0) {
      if (typeof data.workspaceId !== "string") {
        const err21 = { instancePath: instancePath + "/workspaceId", schemaPath: "#/properties/workspaceId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err21];
        } else {
          vErrors.push(err21);
        }
        errors++;
      }
    }
    if (data.workspaceSemanticIrDigest !== void 0) {
      if (typeof data.workspaceSemanticIrDigest !== "string") {
        const err22 = { instancePath: instancePath + "/workspaceSemanticIrDigest", schemaPath: "#/properties/workspaceSemanticIrDigest/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err22];
        } else {
          vErrors.push(err22);
        }
        errors++;
      }
    }
  } else {
    const err23 = { instancePath, schemaPath: "#/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err23];
    } else {
      vErrors.push(err23);
    }
    errors++;
  }
  validate144.errors = vErrors;
  return errors === 0;
}
validate144.evaluated = { "props": true, "dynamicProps": false, "dynamicItems": false };
function validate150(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate150.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  const _errs0 = errors;
  let valid0 = false;
  let passing0 = null;
  const _errs1 = errors;
  if (typeof data !== "string") {
    const err0 = { instancePath, schemaPath: "#/oneOf/0/type", keyword: "type", params: { type: "string" }, message: "must be string" };
    if (vErrors === null) {
      vErrors = [err0];
    } else {
      vErrors.push(err0);
    }
    errors++;
  }
  if ("operation" !== data) {
    const err1 = { instancePath, schemaPath: "#/oneOf/0/const", keyword: "const", params: { allowedValue: "operation" }, message: "must be equal to constant" };
    if (vErrors === null) {
      vErrors = [err1];
    } else {
      vErrors.push(err1);
    }
    errors++;
  }
  var _valid0 = _errs1 === errors;
  if (_valid0) {
    valid0 = true;
    passing0 = 0;
  }
  const _errs3 = errors;
  if (typeof data !== "string") {
    const err2 = { instancePath, schemaPath: "#/oneOf/1/type", keyword: "type", params: { type: "string" }, message: "must be string" };
    if (vErrors === null) {
      vErrors = [err2];
    } else {
      vErrors.push(err2);
    }
    errors++;
  }
  if ("role" !== data) {
    const err3 = { instancePath, schemaPath: "#/oneOf/1/const", keyword: "const", params: { allowedValue: "role" }, message: "must be equal to constant" };
    if (vErrors === null) {
      vErrors = [err3];
    } else {
      vErrors.push(err3);
    }
    errors++;
  }
  var _valid0 = _errs3 === errors;
  if (_valid0 && valid0) {
    valid0 = false;
    passing0 = [passing0, 1];
  } else {
    if (_valid0) {
      valid0 = true;
      passing0 = 1;
    }
    const _errs5 = errors;
    if (typeof data !== "string") {
      const err4 = { instancePath, schemaPath: "#/oneOf/2/type", keyword: "type", params: { type: "string" }, message: "must be string" };
      if (vErrors === null) {
        vErrors = [err4];
      } else {
        vErrors.push(err4);
      }
      errors++;
    }
    if ("doctrine" !== data) {
      const err5 = { instancePath, schemaPath: "#/oneOf/2/const", keyword: "const", params: { allowedValue: "doctrine" }, message: "must be equal to constant" };
      if (vErrors === null) {
        vErrors = [err5];
      } else {
        vErrors.push(err5);
      }
      errors++;
    }
    var _valid0 = _errs5 === errors;
    if (_valid0 && valid0) {
      valid0 = false;
      passing0 = [passing0, 2];
    } else {
      if (_valid0) {
        valid0 = true;
        passing0 = 2;
      }
      const _errs7 = errors;
      if (typeof data !== "string") {
        const err6 = { instancePath, schemaPath: "#/oneOf/3/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err6];
        } else {
          vErrors.push(err6);
        }
        errors++;
      }
      if ("startup-policy" !== data) {
        const err7 = { instancePath, schemaPath: "#/oneOf/3/const", keyword: "const", params: { allowedValue: "startup-policy" }, message: "must be equal to constant" };
        if (vErrors === null) {
          vErrors = [err7];
        } else {
          vErrors.push(err7);
        }
        errors++;
      }
      var _valid0 = _errs7 === errors;
      if (_valid0 && valid0) {
        valid0 = false;
        passing0 = [passing0, 3];
      } else {
        if (_valid0) {
          valid0 = true;
          passing0 = 3;
        }
        const _errs9 = errors;
        if (typeof data !== "string") {
          const err8 = { instancePath, schemaPath: "#/oneOf/4/type", keyword: "type", params: { type: "string" }, message: "must be string" };
          if (vErrors === null) {
            vErrors = [err8];
          } else {
            vErrors.push(err8);
          }
          errors++;
        }
        if ("run-mode" !== data) {
          const err9 = { instancePath, schemaPath: "#/oneOf/4/const", keyword: "const", params: { allowedValue: "run-mode" }, message: "must be equal to constant" };
          if (vErrors === null) {
            vErrors = [err9];
          } else {
            vErrors.push(err9);
          }
          errors++;
        }
        var _valid0 = _errs9 === errors;
        if (_valid0 && valid0) {
          valid0 = false;
          passing0 = [passing0, 4];
        } else {
          if (_valid0) {
            valid0 = true;
            passing0 = 4;
          }
          const _errs11 = errors;
          if (typeof data !== "string") {
            const err10 = { instancePath, schemaPath: "#/oneOf/5/type", keyword: "type", params: { type: "string" }, message: "must be string" };
            if (vErrors === null) {
              vErrors = [err10];
            } else {
              vErrors.push(err10);
            }
            errors++;
          }
          if ("stack" !== data) {
            const err11 = { instancePath, schemaPath: "#/oneOf/5/const", keyword: "const", params: { allowedValue: "stack" }, message: "must be equal to constant" };
            if (vErrors === null) {
              vErrors = [err11];
            } else {
              vErrors.push(err11);
            }
            errors++;
          }
          var _valid0 = _errs11 === errors;
          if (_valid0 && valid0) {
            valid0 = false;
            passing0 = [passing0, 5];
          } else {
            if (_valid0) {
              valid0 = true;
              passing0 = 5;
            }
            const _errs13 = errors;
            if (typeof data !== "string") {
              const err12 = { instancePath, schemaPath: "#/oneOf/6/type", keyword: "type", params: { type: "string" }, message: "must be string" };
              if (vErrors === null) {
                vErrors = [err12];
              } else {
                vErrors.push(err12);
              }
              errors++;
            }
            if ("policy-family" !== data) {
              const err13 = { instancePath, schemaPath: "#/oneOf/6/const", keyword: "const", params: { allowedValue: "policy-family" }, message: "must be equal to constant" };
              if (vErrors === null) {
                vErrors = [err13];
              } else {
                vErrors.push(err13);
              }
              errors++;
            }
            var _valid0 = _errs13 === errors;
            if (_valid0 && valid0) {
              valid0 = false;
              passing0 = [passing0, 6];
            } else {
              if (_valid0) {
                valid0 = true;
                passing0 = 6;
              }
              const _errs15 = errors;
              if (typeof data !== "string") {
                const err14 = { instancePath, schemaPath: "#/oneOf/7/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                if (vErrors === null) {
                  vErrors = [err14];
                } else {
                  vErrors.push(err14);
                }
                errors++;
              }
              if ("recipe" !== data) {
                const err15 = { instancePath, schemaPath: "#/oneOf/7/const", keyword: "const", params: { allowedValue: "recipe" }, message: "must be equal to constant" };
                if (vErrors === null) {
                  vErrors = [err15];
                } else {
                  vErrors.push(err15);
                }
                errors++;
              }
              var _valid0 = _errs15 === errors;
              if (_valid0 && valid0) {
                valid0 = false;
                passing0 = [passing0, 7];
              } else {
                if (_valid0) {
                  valid0 = true;
                  passing0 = 7;
                }
              }
            }
          }
        }
      }
    }
  }
  if (!valid0) {
    const err16 = { instancePath, schemaPath: "#/oneOf", keyword: "oneOf", params: { passingSchemas: passing0 }, message: "must match exactly one schema in oneOf" };
    if (vErrors === null) {
      vErrors = [err16];
    } else {
      vErrors.push(err16);
    }
    errors++;
  } else {
    errors = _errs0;
    if (vErrors !== null) {
      if (_errs0) {
        vErrors.length = _errs0;
      } else {
        vErrors = null;
      }
    }
  }
  validate150.errors = vErrors;
  return errors === 0;
}
validate150.evaluated = { "dynamicProps": false, "dynamicItems": false };
function validate152(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate152.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  const _errs0 = errors;
  let valid0 = false;
  let passing0 = null;
  const _errs1 = errors;
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.kind === void 0) {
      const err0 = { instancePath, schemaPath: "#/oneOf/0/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
      if (vErrors === null) {
        vErrors = [err0];
      } else {
        vErrors.push(err0);
      }
      errors++;
    }
    if (data.resourceId === void 0) {
      const err1 = { instancePath, schemaPath: "#/oneOf/0/required", keyword: "required", params: { missingProperty: "resourceId" }, message: "must have required property 'resourceId'" };
      if (vErrors === null) {
        vErrors = [err1];
      } else {
        vErrors.push(err1);
      }
      errors++;
    }
    if (data.kind !== void 0) {
      let data0 = data.kind;
      if (typeof data0 !== "string") {
        const err2 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/0/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err2];
        } else {
          vErrors.push(err2);
        }
        errors++;
      }
      if ("exact" !== data0) {
        const err3 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/0/properties/kind/const", keyword: "const", params: { allowedValue: "exact" }, message: "must be equal to constant" };
        if (vErrors === null) {
          vErrors = [err3];
        } else {
          vErrors.push(err3);
        }
        errors++;
      }
    }
    if (data.resourceId !== void 0) {
      if (typeof data.resourceId !== "string") {
        const err4 = { instancePath: instancePath + "/resourceId", schemaPath: "#/oneOf/0/properties/resourceId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err4];
        } else {
          vErrors.push(err4);
        }
        errors++;
      }
    }
  } else {
    const err5 = { instancePath, schemaPath: "#/oneOf/0/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err5];
    } else {
      vErrors.push(err5);
    }
    errors++;
  }
  var _valid0 = _errs1 === errors;
  if (_valid0) {
    valid0 = true;
    passing0 = 0;
    var props0 = {};
    props0.kind = true;
    props0.resourceId = true;
  }
  const _errs7 = errors;
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.kind === void 0) {
      const err6 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
      if (vErrors === null) {
        vErrors = [err6];
      } else {
        vErrors.push(err6);
      }
      errors++;
    }
    if (data.packageId === void 0) {
      const err7 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "packageId" }, message: "must have required property 'packageId'" };
      if (vErrors === null) {
        vErrors = [err7];
      } else {
        vErrors.push(err7);
      }
      errors++;
    }
    if (data.kind !== void 0) {
      let data2 = data.kind;
      if (typeof data2 !== "string") {
        const err8 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/1/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err8];
        } else {
          vErrors.push(err8);
        }
        errors++;
      }
      if ("all-from-package" !== data2) {
        const err9 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/1/properties/kind/const", keyword: "const", params: { allowedValue: "all-from-package" }, message: "must be equal to constant" };
        if (vErrors === null) {
          vErrors = [err9];
        } else {
          vErrors.push(err9);
        }
        errors++;
      }
    }
    if (data.packageId !== void 0) {
      if (typeof data.packageId !== "string") {
        const err10 = { instancePath: instancePath + "/packageId", schemaPath: "#/oneOf/1/properties/packageId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err10];
        } else {
          vErrors.push(err10);
        }
        errors++;
      }
    }
  } else {
    const err11 = { instancePath, schemaPath: "#/oneOf/1/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err11];
    } else {
      vErrors.push(err11);
    }
    errors++;
  }
  var _valid0 = _errs7 === errors;
  if (_valid0 && valid0) {
    valid0 = false;
    passing0 = [passing0, 1];
  } else {
    if (_valid0) {
      valid0 = true;
      passing0 = 1;
      if (props0 !== true) {
        props0 = props0 || {};
        props0.kind = true;
        props0.packageId = true;
      }
    }
  }
  if (!valid0) {
    const err12 = { instancePath, schemaPath: "#/oneOf", keyword: "oneOf", params: { passingSchemas: passing0 }, message: "must match exactly one schema in oneOf" };
    if (vErrors === null) {
      vErrors = [err12];
    } else {
      vErrors.push(err12);
    }
    errors++;
  } else {
    errors = _errs0;
    if (vErrors !== null) {
      if (_errs0) {
        vErrors.length = _errs0;
      } else {
        vErrors = null;
      }
    }
  }
  validate152.errors = vErrors;
  evaluated0.props = props0;
  return errors === 0;
}
validate152.evaluated = { "dynamicProps": true, "dynamicItems": false };
function validate154(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate154.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  const _errs0 = errors;
  let valid0 = false;
  let passing0 = null;
  const _errs1 = errors;
  if (typeof data !== "string") {
    const err0 = { instancePath, schemaPath: "#/oneOf/0/type", keyword: "type", params: { type: "string" }, message: "must be string" };
    if (vErrors === null) {
      vErrors = [err0];
    } else {
      vErrors.push(err0);
    }
    errors++;
  }
  if ("intensity" !== data) {
    const err1 = { instancePath, schemaPath: "#/oneOf/0/const", keyword: "const", params: { allowedValue: "intensity" }, message: "must be equal to constant" };
    if (vErrors === null) {
      vErrors = [err1];
    } else {
      vErrors.push(err1);
    }
    errors++;
  }
  var _valid0 = _errs1 === errors;
  if (_valid0) {
    valid0 = true;
    passing0 = 0;
  }
  const _errs3 = errors;
  if (typeof data !== "string") {
    const err2 = { instancePath, schemaPath: "#/oneOf/1/type", keyword: "type", params: { type: "string" }, message: "must be string" };
    if (vErrors === null) {
      vErrors = [err2];
    } else {
      vErrors.push(err2);
    }
    errors++;
  }
  if ("task-stance" !== data) {
    const err3 = { instancePath, schemaPath: "#/oneOf/1/const", keyword: "const", params: { allowedValue: "task-stance" }, message: "must be equal to constant" };
    if (vErrors === null) {
      vErrors = [err3];
    } else {
      vErrors.push(err3);
    }
    errors++;
  }
  var _valid0 = _errs3 === errors;
  if (_valid0 && valid0) {
    valid0 = false;
    passing0 = [passing0, 1];
  } else {
    if (_valid0) {
      valid0 = true;
      passing0 = 1;
    }
  }
  if (!valid0) {
    const err4 = { instancePath, schemaPath: "#/oneOf", keyword: "oneOf", params: { passingSchemas: passing0 }, message: "must match exactly one schema in oneOf" };
    if (vErrors === null) {
      vErrors = [err4];
    } else {
      vErrors.push(err4);
    }
    errors++;
  } else {
    errors = _errs0;
    if (vErrors !== null) {
      if (_errs0) {
        vErrors.length = _errs0;
      } else {
        vErrors = null;
      }
    }
  }
  validate154.errors = vErrors;
  return errors === 0;
}
validate154.evaluated = { "dynamicProps": false, "dynamicItems": false };
function validate149(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate149.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  const _errs0 = errors;
  let valid0 = false;
  let passing0 = null;
  const _errs1 = errors;
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.kind === void 0) {
      const err0 = { instancePath, schemaPath: "#/oneOf/0/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
      if (vErrors === null) {
        vErrors = [err0];
      } else {
        vErrors.push(err0);
      }
      errors++;
    }
    if (data.resourceKind === void 0) {
      const err1 = { instancePath, schemaPath: "#/oneOf/0/required", keyword: "required", params: { missingProperty: "resourceKind" }, message: "must have required property 'resourceKind'" };
      if (vErrors === null) {
        vErrors = [err1];
      } else {
        vErrors.push(err1);
      }
      errors++;
    }
    if (data.selector === void 0) {
      const err2 = { instancePath, schemaPath: "#/oneOf/0/required", keyword: "required", params: { missingProperty: "selector" }, message: "must have required property 'selector'" };
      if (vErrors === null) {
        vErrors = [err2];
      } else {
        vErrors.push(err2);
      }
      errors++;
    }
    if (data.kind !== void 0) {
      let data0 = data.kind;
      if (typeof data0 !== "string") {
        const err3 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/0/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err3];
        } else {
          vErrors.push(err3);
        }
        errors++;
      }
      if ("resource" !== data0) {
        const err4 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/0/properties/kind/const", keyword: "const", params: { allowedValue: "resource" }, message: "must be equal to constant" };
        if (vErrors === null) {
          vErrors = [err4];
        } else {
          vErrors.push(err4);
        }
        errors++;
      }
    }
    if (data.resourceKind !== void 0) {
      if (!validate150(data.resourceKind, { instancePath: instancePath + "/resourceKind", parentData: data, parentDataProperty: "resourceKind", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate150.errors : vErrors.concat(validate150.errors);
        errors = vErrors.length;
      }
    }
    if (data.selector !== void 0) {
      if (!validate152(data.selector, { instancePath: instancePath + "/selector", parentData: data, parentDataProperty: "selector", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate152.errors : vErrors.concat(validate152.errors);
        errors = vErrors.length;
      }
    }
  } else {
    const err5 = { instancePath, schemaPath: "#/oneOf/0/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err5];
    } else {
      vErrors.push(err5);
    }
    errors++;
  }
  var _valid0 = _errs1 === errors;
  if (_valid0) {
    valid0 = true;
    passing0 = 0;
    var props1 = {};
    props1.kind = true;
    props1.resourceKind = true;
    props1.selector = true;
  }
  const _errs7 = errors;
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.kind === void 0) {
      const err6 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
      if (vErrors === null) {
        vErrors = [err6];
      } else {
        vErrors.push(err6);
      }
      errors++;
    }
    if (data.axis === void 0) {
      const err7 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "axis" }, message: "must have required property 'axis'" };
      if (vErrors === null) {
        vErrors = [err7];
      } else {
        vErrors.push(err7);
      }
      errors++;
    }
    if (data.resourceId === void 0) {
      const err8 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "resourceId" }, message: "must have required property 'resourceId'" };
      if (vErrors === null) {
        vErrors = [err8];
      } else {
        vErrors.push(err8);
      }
      errors++;
    }
    if (data.axis !== void 0) {
      if (!validate154(data.axis, { instancePath: instancePath + "/axis", parentData: data, parentDataProperty: "axis", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate154.errors : vErrors.concat(validate154.errors);
        errors = vErrors.length;
      }
    }
    if (data.kind !== void 0) {
      let data4 = data.kind;
      if (typeof data4 !== "string") {
        const err9 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/1/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err9];
        } else {
          vErrors.push(err9);
        }
        errors++;
      }
      if ("mode-default" !== data4) {
        const err10 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/1/properties/kind/const", keyword: "const", params: { allowedValue: "mode-default" }, message: "must be equal to constant" };
        if (vErrors === null) {
          vErrors = [err10];
        } else {
          vErrors.push(err10);
        }
        errors++;
      }
    }
    if (data.resourceId !== void 0) {
      if (typeof data.resourceId !== "string") {
        const err11 = { instancePath: instancePath + "/resourceId", schemaPath: "#/oneOf/1/properties/resourceId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err11];
        } else {
          vErrors.push(err11);
        }
        errors++;
      }
    }
  } else {
    const err12 = { instancePath, schemaPath: "#/oneOf/1/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err12];
    } else {
      vErrors.push(err12);
    }
    errors++;
  }
  var _valid0 = _errs7 === errors;
  if (_valid0 && valid0) {
    valid0 = false;
    passing0 = [passing0, 1];
  } else {
    if (_valid0) {
      valid0 = true;
      passing0 = 1;
      if (props1 !== true) {
        props1 = props1 || {};
        props1.axis = true;
        props1.kind = true;
        props1.resourceId = true;
      }
    }
  }
  if (!valid0) {
    const err13 = { instancePath, schemaPath: "#/oneOf", keyword: "oneOf", params: { passingSchemas: passing0 }, message: "must match exactly one schema in oneOf" };
    if (vErrors === null) {
      vErrors = [err13];
    } else {
      vErrors.push(err13);
    }
    errors++;
  } else {
    errors = _errs0;
    if (vErrors !== null) {
      if (_errs0) {
        vErrors.length = _errs0;
      } else {
        vErrors = null;
      }
    }
  }
  validate149.errors = vErrors;
  evaluated0.props = props1;
  return errors === 0;
}
validate149.evaluated = { "dynamicProps": true, "dynamicItems": false };
function validate157(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate157.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  const _errs0 = errors;
  let valid0 = false;
  let passing0 = null;
  const _errs1 = errors;
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.kind === void 0) {
      const err0 = { instancePath, schemaPath: "#/oneOf/0/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
      if (vErrors === null) {
        vErrors = [err0];
      } else {
        vErrors.push(err0);
      }
      errors++;
    }
    if (data.contractId === void 0) {
      const err1 = { instancePath, schemaPath: "#/oneOf/0/required", keyword: "required", params: { missingProperty: "contractId" }, message: "must have required property 'contractId'" };
      if (vErrors === null) {
        vErrors = [err1];
      } else {
        vErrors.push(err1);
      }
      errors++;
    }
    if (data.contractId !== void 0) {
      if (typeof data.contractId !== "string") {
        const err2 = { instancePath: instancePath + "/contractId", schemaPath: "#/oneOf/0/properties/contractId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err2];
        } else {
          vErrors.push(err2);
        }
        errors++;
      }
    }
    if (data.kind !== void 0) {
      let data1 = data.kind;
      if (typeof data1 !== "string") {
        const err3 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/0/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err3];
        } else {
          vErrors.push(err3);
        }
        errors++;
      }
      if ("origin-receipt" !== data1) {
        const err4 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/0/properties/kind/const", keyword: "const", params: { allowedValue: "origin-receipt" }, message: "must be equal to constant" };
        if (vErrors === null) {
          vErrors = [err4];
        } else {
          vErrors.push(err4);
        }
        errors++;
      }
    }
  } else {
    const err5 = { instancePath, schemaPath: "#/oneOf/0/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err5];
    } else {
      vErrors.push(err5);
    }
    errors++;
  }
  var _valid0 = _errs1 === errors;
  if (_valid0) {
    valid0 = true;
    passing0 = 0;
    var props0 = {};
    props0.contractId = true;
    props0.kind = true;
  }
  const _errs7 = errors;
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.kind === void 0) {
      const err6 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
      if (vErrors === null) {
        vErrors = [err6];
      } else {
        vErrors.push(err6);
      }
      errors++;
    }
    if (data.contractId === void 0) {
      const err7 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "contractId" }, message: "must have required property 'contractId'" };
      if (vErrors === null) {
        vErrors = [err7];
      } else {
        vErrors.push(err7);
      }
      errors++;
    }
    if (data.contractId !== void 0) {
      if (typeof data.contractId !== "string") {
        const err8 = { instancePath: instancePath + "/contractId", schemaPath: "#/oneOf/1/properties/contractId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err8];
        } else {
          vErrors.push(err8);
        }
        errors++;
      }
    }
    if (data.kind !== void 0) {
      let data3 = data.kind;
      if (typeof data3 !== "string") {
        const err9 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/1/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err9];
        } else {
          vErrors.push(err9);
        }
        errors++;
      }
      if ("entry-contract" !== data3) {
        const err10 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/1/properties/kind/const", keyword: "const", params: { allowedValue: "entry-contract" }, message: "must be equal to constant" };
        if (vErrors === null) {
          vErrors = [err10];
        } else {
          vErrors.push(err10);
        }
        errors++;
      }
    }
  } else {
    const err11 = { instancePath, schemaPath: "#/oneOf/1/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err11];
    } else {
      vErrors.push(err11);
    }
    errors++;
  }
  var _valid0 = _errs7 === errors;
  if (_valid0 && valid0) {
    valid0 = false;
    passing0 = [passing0, 1];
  } else {
    if (_valid0) {
      valid0 = true;
      passing0 = 1;
      if (props0 !== true) {
        props0 = props0 || {};
        props0.contractId = true;
        props0.kind = true;
      }
    }
    const _errs13 = errors;
    if (data && typeof data == "object" && !Array.isArray(data)) {
      if (data.kind === void 0) {
        const err12 = { instancePath, schemaPath: "#/oneOf/2/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
        if (vErrors === null) {
          vErrors = [err12];
        } else {
          vErrors.push(err12);
        }
        errors++;
      }
      if (data.contractId === void 0) {
        const err13 = { instancePath, schemaPath: "#/oneOf/2/required", keyword: "required", params: { missingProperty: "contractId" }, message: "must have required property 'contractId'" };
        if (vErrors === null) {
          vErrors = [err13];
        } else {
          vErrors.push(err13);
        }
        errors++;
      }
      if (data.contractId !== void 0) {
        if (typeof data.contractId !== "string") {
          const err14 = { instancePath: instancePath + "/contractId", schemaPath: "#/oneOf/2/properties/contractId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
          if (vErrors === null) {
            vErrors = [err14];
          } else {
            vErrors.push(err14);
          }
          errors++;
        }
      }
      if (data.kind !== void 0) {
        let data5 = data.kind;
        if (typeof data5 !== "string") {
          const err15 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/2/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
          if (vErrors === null) {
            vErrors = [err15];
          } else {
            vErrors.push(err15);
          }
          errors++;
        }
        if ("responsibility-contract" !== data5) {
          const err16 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/2/properties/kind/const", keyword: "const", params: { allowedValue: "responsibility-contract" }, message: "must be equal to constant" };
          if (vErrors === null) {
            vErrors = [err16];
          } else {
            vErrors.push(err16);
          }
          errors++;
        }
      }
    } else {
      const err17 = { instancePath, schemaPath: "#/oneOf/2/type", keyword: "type", params: { type: "object" }, message: "must be object" };
      if (vErrors === null) {
        vErrors = [err17];
      } else {
        vErrors.push(err17);
      }
      errors++;
    }
    var _valid0 = _errs13 === errors;
    if (_valid0 && valid0) {
      valid0 = false;
      passing0 = [passing0, 2];
    } else {
      if (_valid0) {
        valid0 = true;
        passing0 = 2;
        if (props0 !== true) {
          props0 = props0 || {};
          props0.contractId = true;
          props0.kind = true;
        }
      }
      const _errs19 = errors;
      if (data && typeof data == "object" && !Array.isArray(data)) {
        if (data.kind === void 0) {
          const err18 = { instancePath, schemaPath: "#/oneOf/3/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
          if (vErrors === null) {
            vErrors = [err18];
          } else {
            vErrors.push(err18);
          }
          errors++;
        }
        if (data.bindingId === void 0) {
          const err19 = { instancePath, schemaPath: "#/oneOf/3/required", keyword: "required", params: { missingProperty: "bindingId" }, message: "must have required property 'bindingId'" };
          if (vErrors === null) {
            vErrors = [err19];
          } else {
            vErrors.push(err19);
          }
          errors++;
        }
        if (data.bindingId !== void 0) {
          if (typeof data.bindingId !== "string") {
            const err20 = { instancePath: instancePath + "/bindingId", schemaPath: "#/oneOf/3/properties/bindingId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
            if (vErrors === null) {
              vErrors = [err20];
            } else {
              vErrors.push(err20);
            }
            errors++;
          }
        }
        if (data.kind !== void 0) {
          let data7 = data.kind;
          if (typeof data7 !== "string") {
            const err21 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/3/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
            if (vErrors === null) {
              vErrors = [err21];
            } else {
              vErrors.push(err21);
            }
            errors++;
          }
          if ("provider-binding" !== data7) {
            const err22 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/3/properties/kind/const", keyword: "const", params: { allowedValue: "provider-binding" }, message: "must be equal to constant" };
            if (vErrors === null) {
              vErrors = [err22];
            } else {
              vErrors.push(err22);
            }
            errors++;
          }
        }
      } else {
        const err23 = { instancePath, schemaPath: "#/oneOf/3/type", keyword: "type", params: { type: "object" }, message: "must be object" };
        if (vErrors === null) {
          vErrors = [err23];
        } else {
          vErrors.push(err23);
        }
        errors++;
      }
      var _valid0 = _errs19 === errors;
      if (_valid0 && valid0) {
        valid0 = false;
        passing0 = [passing0, 3];
      } else {
        if (_valid0) {
          valid0 = true;
          passing0 = 3;
          if (props0 !== true) {
            props0 = props0 || {};
            props0.bindingId = true;
            props0.kind = true;
          }
        }
        const _errs25 = errors;
        if (data && typeof data == "object" && !Array.isArray(data)) {
          if (data.kind === void 0) {
            const err24 = { instancePath, schemaPath: "#/oneOf/4/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
            if (vErrors === null) {
              vErrors = [err24];
            } else {
              vErrors.push(err24);
            }
            errors++;
          }
          if (data.edgeId === void 0) {
            const err25 = { instancePath, schemaPath: "#/oneOf/4/required", keyword: "required", params: { missingProperty: "edgeId" }, message: "must have required property 'edgeId'" };
            if (vErrors === null) {
              vErrors = [err25];
            } else {
              vErrors.push(err25);
            }
            errors++;
          }
          if (data.edgeId !== void 0) {
            if (typeof data.edgeId !== "string") {
              const err26 = { instancePath: instancePath + "/edgeId", schemaPath: "#/oneOf/4/properties/edgeId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
              if (vErrors === null) {
                vErrors = [err26];
              } else {
                vErrors.push(err26);
              }
              errors++;
            }
          }
          if (data.kind !== void 0) {
            let data9 = data.kind;
            if (typeof data9 !== "string") {
              const err27 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/4/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
              if (vErrors === null) {
                vErrors = [err27];
              } else {
                vErrors.push(err27);
              }
              errors++;
            }
            if ("delegation-edge" !== data9) {
              const err28 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/4/properties/kind/const", keyword: "const", params: { allowedValue: "delegation-edge" }, message: "must be equal to constant" };
              if (vErrors === null) {
                vErrors = [err28];
              } else {
                vErrors.push(err28);
              }
              errors++;
            }
          }
        } else {
          const err29 = { instancePath, schemaPath: "#/oneOf/4/type", keyword: "type", params: { type: "object" }, message: "must be object" };
          if (vErrors === null) {
            vErrors = [err29];
          } else {
            vErrors.push(err29);
          }
          errors++;
        }
        var _valid0 = _errs25 === errors;
        if (_valid0 && valid0) {
          valid0 = false;
          passing0 = [passing0, 4];
        } else {
          if (_valid0) {
            valid0 = true;
            passing0 = 4;
            if (props0 !== true) {
              props0 = props0 || {};
              props0.edgeId = true;
              props0.kind = true;
            }
          }
          const _errs31 = errors;
          if (data && typeof data == "object" && !Array.isArray(data)) {
            if (data.kind === void 0) {
              const err30 = { instancePath, schemaPath: "#/oneOf/5/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
              if (vErrors === null) {
                vErrors = [err30];
              } else {
                vErrors.push(err30);
              }
              errors++;
            }
            if (data.contractId === void 0) {
              const err31 = { instancePath, schemaPath: "#/oneOf/5/required", keyword: "required", params: { missingProperty: "contractId" }, message: "must have required property 'contractId'" };
              if (vErrors === null) {
                vErrors = [err31];
              } else {
                vErrors.push(err31);
              }
              errors++;
            }
            if (data.contractId !== void 0) {
              if (typeof data.contractId !== "string") {
                const err32 = { instancePath: instancePath + "/contractId", schemaPath: "#/oneOf/5/properties/contractId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                if (vErrors === null) {
                  vErrors = [err32];
                } else {
                  vErrors.push(err32);
                }
                errors++;
              }
            }
            if (data.kind !== void 0) {
              let data11 = data.kind;
              if (typeof data11 !== "string") {
                const err33 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/5/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                if (vErrors === null) {
                  vErrors = [err33];
                } else {
                  vErrors.push(err33);
                }
                errors++;
              }
              if ("termination-contract" !== data11) {
                const err34 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/5/properties/kind/const", keyword: "const", params: { allowedValue: "termination-contract" }, message: "must be equal to constant" };
                if (vErrors === null) {
                  vErrors = [err34];
                } else {
                  vErrors.push(err34);
                }
                errors++;
              }
            }
          } else {
            const err35 = { instancePath, schemaPath: "#/oneOf/5/type", keyword: "type", params: { type: "object" }, message: "must be object" };
            if (vErrors === null) {
              vErrors = [err35];
            } else {
              vErrors.push(err35);
            }
            errors++;
          }
          var _valid0 = _errs31 === errors;
          if (_valid0 && valid0) {
            valid0 = false;
            passing0 = [passing0, 5];
          } else {
            if (_valid0) {
              valid0 = true;
              passing0 = 5;
              if (props0 !== true) {
                props0 = props0 || {};
                props0.contractId = true;
                props0.kind = true;
              }
            }
            const _errs37 = errors;
            if (data && typeof data == "object" && !Array.isArray(data)) {
              if (data.kind === void 0) {
                const err36 = { instancePath, schemaPath: "#/oneOf/6/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
                if (vErrors === null) {
                  vErrors = [err36];
                } else {
                  vErrors.push(err36);
                }
                errors++;
              }
              if (data.contractId === void 0) {
                const err37 = { instancePath, schemaPath: "#/oneOf/6/required", keyword: "required", params: { missingProperty: "contractId" }, message: "must have required property 'contractId'" };
                if (vErrors === null) {
                  vErrors = [err37];
                } else {
                  vErrors.push(err37);
                }
                errors++;
              }
              if (data.contractId !== void 0) {
                if (typeof data.contractId !== "string") {
                  const err38 = { instancePath: instancePath + "/contractId", schemaPath: "#/oneOf/6/properties/contractId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                  if (vErrors === null) {
                    vErrors = [err38];
                  } else {
                    vErrors.push(err38);
                  }
                  errors++;
                }
              }
              if (data.kind !== void 0) {
                let data13 = data.kind;
                if (typeof data13 !== "string") {
                  const err39 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/6/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                  if (vErrors === null) {
                    vErrors = [err39];
                  } else {
                    vErrors.push(err39);
                  }
                  errors++;
                }
                if ("settlement-contract" !== data13) {
                  const err40 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/6/properties/kind/const", keyword: "const", params: { allowedValue: "settlement-contract" }, message: "must be equal to constant" };
                  if (vErrors === null) {
                    vErrors = [err40];
                  } else {
                    vErrors.push(err40);
                  }
                  errors++;
                }
              }
            } else {
              const err41 = { instancePath, schemaPath: "#/oneOf/6/type", keyword: "type", params: { type: "object" }, message: "must be object" };
              if (vErrors === null) {
                vErrors = [err41];
              } else {
                vErrors.push(err41);
              }
              errors++;
            }
            var _valid0 = _errs37 === errors;
            if (_valid0 && valid0) {
              valid0 = false;
              passing0 = [passing0, 6];
            } else {
              if (_valid0) {
                valid0 = true;
                passing0 = 6;
                if (props0 !== true) {
                  props0 = props0 || {};
                  props0.contractId = true;
                  props0.kind = true;
                }
              }
              const _errs43 = errors;
              if (data && typeof data == "object" && !Array.isArray(data)) {
                if (data.kind === void 0) {
                  const err42 = { instancePath, schemaPath: "#/oneOf/7/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
                  if (vErrors === null) {
                    vErrors = [err42];
                  } else {
                    vErrors.push(err42);
                  }
                  errors++;
                }
                if (data.contractId === void 0) {
                  const err43 = { instancePath, schemaPath: "#/oneOf/7/required", keyword: "required", params: { missingProperty: "contractId" }, message: "must have required property 'contractId'" };
                  if (vErrors === null) {
                    vErrors = [err43];
                  } else {
                    vErrors.push(err43);
                  }
                  errors++;
                }
                if (data.contractId !== void 0) {
                  if (typeof data.contractId !== "string") {
                    const err44 = { instancePath: instancePath + "/contractId", schemaPath: "#/oneOf/7/properties/contractId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                    if (vErrors === null) {
                      vErrors = [err44];
                    } else {
                      vErrors.push(err44);
                    }
                    errors++;
                  }
                }
                if (data.kind !== void 0) {
                  let data15 = data.kind;
                  if (typeof data15 !== "string") {
                    const err45 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/7/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                    if (vErrors === null) {
                      vErrors = [err45];
                    } else {
                      vErrors.push(err45);
                    }
                    errors++;
                  }
                  if ("continuity-contract" !== data15) {
                    const err46 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/7/properties/kind/const", keyword: "const", params: { allowedValue: "continuity-contract" }, message: "must be equal to constant" };
                    if (vErrors === null) {
                      vErrors = [err46];
                    } else {
                      vErrors.push(err46);
                    }
                    errors++;
                  }
                }
              } else {
                const err47 = { instancePath, schemaPath: "#/oneOf/7/type", keyword: "type", params: { type: "object" }, message: "must be object" };
                if (vErrors === null) {
                  vErrors = [err47];
                } else {
                  vErrors.push(err47);
                }
                errors++;
              }
              var _valid0 = _errs43 === errors;
              if (_valid0 && valid0) {
                valid0 = false;
                passing0 = [passing0, 7];
              } else {
                if (_valid0) {
                  valid0 = true;
                  passing0 = 7;
                  if (props0 !== true) {
                    props0 = props0 || {};
                    props0.contractId = true;
                    props0.kind = true;
                  }
                }
              }
            }
          }
        }
      }
    }
  }
  if (!valid0) {
    const err48 = { instancePath, schemaPath: "#/oneOf", keyword: "oneOf", params: { passingSchemas: passing0 }, message: "must match exactly one schema in oneOf" };
    if (vErrors === null) {
      vErrors = [err48];
    } else {
      vErrors.push(err48);
    }
    errors++;
  } else {
    errors = _errs0;
    if (vErrors !== null) {
      if (_errs0) {
        vErrors.length = _errs0;
      } else {
        vErrors = null;
      }
    }
  }
  validate157.errors = vErrors;
  evaluated0.props = props0;
  return errors === 0;
}
validate157.evaluated = { "dynamicProps": true, "dynamicItems": false };
function validate159(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate159.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  const _errs0 = errors;
  let valid0 = false;
  let passing0 = null;
  const _errs1 = errors;
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.kind === void 0) {
      const err0 = { instancePath, schemaPath: "#/oneOf/0/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
      if (vErrors === null) {
        vErrors = [err0];
      } else {
        vErrors.push(err0);
      }
      errors++;
    }
    if (data.packageId === void 0) {
      const err1 = { instancePath, schemaPath: "#/oneOf/0/required", keyword: "required", params: { missingProperty: "packageId" }, message: "must have required property 'packageId'" };
      if (vErrors === null) {
        vErrors = [err1];
      } else {
        vErrors.push(err1);
      }
      errors++;
    }
    if (data.version === void 0) {
      const err2 = { instancePath, schemaPath: "#/oneOf/0/required", keyword: "required", params: { missingProperty: "version" }, message: "must have required property 'version'" };
      if (vErrors === null) {
        vErrors = [err2];
      } else {
        vErrors.push(err2);
      }
      errors++;
    }
    if (data.contentDigest === void 0) {
      const err3 = { instancePath, schemaPath: "#/oneOf/0/required", keyword: "required", params: { missingProperty: "contentDigest" }, message: "must have required property 'contentDigest'" };
      if (vErrors === null) {
        vErrors = [err3];
      } else {
        vErrors.push(err3);
      }
      errors++;
    }
    if (data.contentDigest !== void 0) {
      if (typeof data.contentDigest !== "string") {
        const err4 = { instancePath: instancePath + "/contentDigest", schemaPath: "#/oneOf/0/properties/contentDigest/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err4];
        } else {
          vErrors.push(err4);
        }
        errors++;
      }
    }
    if (data.kind !== void 0) {
      let data1 = data.kind;
      if (typeof data1 !== "string") {
        const err5 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/0/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err5];
        } else {
          vErrors.push(err5);
        }
        errors++;
      }
      if ("package" !== data1) {
        const err6 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/0/properties/kind/const", keyword: "const", params: { allowedValue: "package" }, message: "must be equal to constant" };
        if (vErrors === null) {
          vErrors = [err6];
        } else {
          vErrors.push(err6);
        }
        errors++;
      }
    }
    if (data.packageId !== void 0) {
      if (typeof data.packageId !== "string") {
        const err7 = { instancePath: instancePath + "/packageId", schemaPath: "#/oneOf/0/properties/packageId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err7];
        } else {
          vErrors.push(err7);
        }
        errors++;
      }
    }
    if (data.version !== void 0) {
      if (typeof data.version !== "string") {
        const err8 = { instancePath: instancePath + "/version", schemaPath: "#/oneOf/0/properties/version/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err8];
        } else {
          vErrors.push(err8);
        }
        errors++;
      }
    }
  } else {
    const err9 = { instancePath, schemaPath: "#/oneOf/0/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err9];
    } else {
      vErrors.push(err9);
    }
    errors++;
  }
  var _valid0 = _errs1 === errors;
  if (_valid0) {
    valid0 = true;
    passing0 = 0;
    var props0 = {};
    props0.contentDigest = true;
    props0.kind = true;
    props0.packageId = true;
    props0.version = true;
  }
  const _errs11 = errors;
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.kind === void 0) {
      const err10 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
      if (vErrors === null) {
        vErrors = [err10];
      } else {
        vErrors.push(err10);
      }
      errors++;
    }
    if (data.packId === void 0) {
      const err11 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "packId" }, message: "must have required property 'packId'" };
      if (vErrors === null) {
        vErrors = [err11];
      } else {
        vErrors.push(err11);
      }
      errors++;
    }
    if (data.attestorId === void 0) {
      const err12 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "attestorId" }, message: "must have required property 'attestorId'" };
      if (vErrors === null) {
        vErrors = [err12];
      } else {
        vErrors.push(err12);
      }
      errors++;
    }
    if (data.version === void 0) {
      const err13 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "version" }, message: "must have required property 'version'" };
      if (vErrors === null) {
        vErrors = [err13];
      } else {
        vErrors.push(err13);
      }
      errors++;
    }
    if (data.contentDigest === void 0) {
      const err14 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "contentDigest" }, message: "must have required property 'contentDigest'" };
      if (vErrors === null) {
        vErrors = [err14];
      } else {
        vErrors.push(err14);
      }
      errors++;
    }
    if (data.attestorId !== void 0) {
      if (typeof data.attestorId !== "string") {
        const err15 = { instancePath: instancePath + "/attestorId", schemaPath: "#/oneOf/1/properties/attestorId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err15];
        } else {
          vErrors.push(err15);
        }
        errors++;
      }
    }
    if (data.contentDigest !== void 0) {
      if (typeof data.contentDigest !== "string") {
        const err16 = { instancePath: instancePath + "/contentDigest", schemaPath: "#/oneOf/1/properties/contentDigest/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err16];
        } else {
          vErrors.push(err16);
        }
        errors++;
      }
    }
    if (data.kind !== void 0) {
      let data6 = data.kind;
      if (typeof data6 !== "string") {
        const err17 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/1/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err17];
        } else {
          vErrors.push(err17);
        }
        errors++;
      }
      if ("normative-pack" !== data6) {
        const err18 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/1/properties/kind/const", keyword: "const", params: { allowedValue: "normative-pack" }, message: "must be equal to constant" };
        if (vErrors === null) {
          vErrors = [err18];
        } else {
          vErrors.push(err18);
        }
        errors++;
      }
    }
    if (data.packId !== void 0) {
      if (typeof data.packId !== "string") {
        const err19 = { instancePath: instancePath + "/packId", schemaPath: "#/oneOf/1/properties/packId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err19];
        } else {
          vErrors.push(err19);
        }
        errors++;
      }
    }
    if (data.version !== void 0) {
      if (typeof data.version !== "string") {
        const err20 = { instancePath: instancePath + "/version", schemaPath: "#/oneOf/1/properties/version/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err20];
        } else {
          vErrors.push(err20);
        }
        errors++;
      }
    }
  } else {
    const err21 = { instancePath, schemaPath: "#/oneOf/1/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err21];
    } else {
      vErrors.push(err21);
    }
    errors++;
  }
  var _valid0 = _errs11 === errors;
  if (_valid0 && valid0) {
    valid0 = false;
    passing0 = [passing0, 1];
  } else {
    if (_valid0) {
      valid0 = true;
      passing0 = 1;
      if (props0 !== true) {
        props0 = props0 || {};
        props0.attestorId = true;
        props0.contentDigest = true;
        props0.kind = true;
        props0.packId = true;
        props0.version = true;
      }
    }
    const _errs23 = errors;
    if (data && typeof data == "object" && !Array.isArray(data)) {
      if (data.kind === void 0) {
        const err22 = { instancePath, schemaPath: "#/oneOf/2/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
        if (vErrors === null) {
          vErrors = [err22];
        } else {
          vErrors.push(err22);
        }
        errors++;
      }
      if (data.repositoryId === void 0) {
        const err23 = { instancePath, schemaPath: "#/oneOf/2/required", keyword: "required", params: { missingProperty: "repositoryId" }, message: "must have required property 'repositoryId'" };
        if (vErrors === null) {
          vErrors = [err23];
        } else {
          vErrors.push(err23);
        }
        errors++;
      }
      if (data.resourceKind === void 0) {
        const err24 = { instancePath, schemaPath: "#/oneOf/2/required", keyword: "required", params: { missingProperty: "resourceKind" }, message: "must have required property 'resourceKind'" };
        if (vErrors === null) {
          vErrors = [err24];
        } else {
          vErrors.push(err24);
        }
        errors++;
      }
      if (data.resourceId === void 0) {
        const err25 = { instancePath, schemaPath: "#/oneOf/2/required", keyword: "required", params: { missingProperty: "resourceId" }, message: "must have required property 'resourceId'" };
        if (vErrors === null) {
          vErrors = [err25];
        } else {
          vErrors.push(err25);
        }
        errors++;
      }
      if (data.kind !== void 0) {
        let data9 = data.kind;
        if (typeof data9 !== "string") {
          const err26 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/2/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
          if (vErrors === null) {
            vErrors = [err26];
          } else {
            vErrors.push(err26);
          }
          errors++;
        }
        if ("semantic-resource" !== data9) {
          const err27 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/2/properties/kind/const", keyword: "const", params: { allowedValue: "semantic-resource" }, message: "must be equal to constant" };
          if (vErrors === null) {
            vErrors = [err27];
          } else {
            vErrors.push(err27);
          }
          errors++;
        }
      }
      if (data.repositoryId !== void 0) {
        if (typeof data.repositoryId !== "string") {
          const err28 = { instancePath: instancePath + "/repositoryId", schemaPath: "#/oneOf/2/properties/repositoryId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
          if (vErrors === null) {
            vErrors = [err28];
          } else {
            vErrors.push(err28);
          }
          errors++;
        }
      }
      if (data.resourceId !== void 0) {
        if (typeof data.resourceId !== "string") {
          const err29 = { instancePath: instancePath + "/resourceId", schemaPath: "#/oneOf/2/properties/resourceId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
          if (vErrors === null) {
            vErrors = [err29];
          } else {
            vErrors.push(err29);
          }
          errors++;
        }
      }
      if (data.resourceKind !== void 0) {
        if (!validate150(data.resourceKind, { instancePath: instancePath + "/resourceKind", parentData: data, parentDataProperty: "resourceKind", rootData, dynamicAnchors })) {
          vErrors = vErrors === null ? validate150.errors : vErrors.concat(validate150.errors);
          errors = vErrors.length;
        }
      }
    } else {
      const err30 = { instancePath, schemaPath: "#/oneOf/2/type", keyword: "type", params: { type: "object" }, message: "must be object" };
      if (vErrors === null) {
        vErrors = [err30];
      } else {
        vErrors.push(err30);
      }
      errors++;
    }
    var _valid0 = _errs23 === errors;
    if (_valid0 && valid0) {
      valid0 = false;
      passing0 = [passing0, 2];
    } else {
      if (_valid0) {
        valid0 = true;
        passing0 = 2;
        if (props0 !== true) {
          props0 = props0 || {};
          props0.kind = true;
          props0.repositoryId = true;
          props0.resourceId = true;
          props0.resourceKind = true;
        }
      }
      const _errs32 = errors;
      if (data && typeof data == "object" && !Array.isArray(data)) {
        if (data.kind === void 0) {
          const err31 = { instancePath, schemaPath: "#/oneOf/3/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
          if (vErrors === null) {
            vErrors = [err31];
          } else {
            vErrors.push(err31);
          }
          errors++;
        }
        if (data.runPolicyId === void 0) {
          const err32 = { instancePath, schemaPath: "#/oneOf/3/required", keyword: "required", params: { missingProperty: "runPolicyId" }, message: "must have required property 'runPolicyId'" };
          if (vErrors === null) {
            vErrors = [err32];
          } else {
            vErrors.push(err32);
          }
          errors++;
        }
        if (data.kind !== void 0) {
          let data13 = data.kind;
          if (typeof data13 !== "string") {
            const err33 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/3/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
            if (vErrors === null) {
              vErrors = [err33];
            } else {
              vErrors.push(err33);
            }
            errors++;
          }
          if ("run-policy" !== data13) {
            const err34 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/3/properties/kind/const", keyword: "const", params: { allowedValue: "run-policy" }, message: "must be equal to constant" };
            if (vErrors === null) {
              vErrors = [err34];
            } else {
              vErrors.push(err34);
            }
            errors++;
          }
        }
        if (data.runPolicyId !== void 0) {
          if (typeof data.runPolicyId !== "string") {
            const err35 = { instancePath: instancePath + "/runPolicyId", schemaPath: "#/oneOf/3/properties/runPolicyId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
            if (vErrors === null) {
              vErrors = [err35];
            } else {
              vErrors.push(err35);
            }
            errors++;
          }
        }
      } else {
        const err36 = { instancePath, schemaPath: "#/oneOf/3/type", keyword: "type", params: { type: "object" }, message: "must be object" };
        if (vErrors === null) {
          vErrors = [err36];
        } else {
          vErrors.push(err36);
        }
        errors++;
      }
      var _valid0 = _errs32 === errors;
      if (_valid0 && valid0) {
        valid0 = false;
        passing0 = [passing0, 3];
      } else {
        if (_valid0) {
          valid0 = true;
          passing0 = 3;
          if (props0 !== true) {
            props0 = props0 || {};
            props0.kind = true;
            props0.runPolicyId = true;
          }
        }
        const _errs38 = errors;
        if (data && typeof data == "object" && !Array.isArray(data)) {
          if (data.kind === void 0) {
            const err37 = { instancePath, schemaPath: "#/oneOf/4/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
            if (vErrors === null) {
              vErrors = [err37];
            } else {
              vErrors.push(err37);
            }
            errors++;
          }
          if (data.executionWorkspaceId === void 0) {
            const err38 = { instancePath, schemaPath: "#/oneOf/4/required", keyword: "required", params: { missingProperty: "executionWorkspaceId" }, message: "must have required property 'executionWorkspaceId'" };
            if (vErrors === null) {
              vErrors = [err38];
            } else {
              vErrors.push(err38);
            }
            errors++;
          }
          if (data.activationRootCount === void 0) {
            const err39 = { instancePath, schemaPath: "#/oneOf/4/required", keyword: "required", params: { missingProperty: "activationRootCount" }, message: "must have required property 'activationRootCount'" };
            if (vErrors === null) {
              vErrors = [err39];
            } else {
              vErrors.push(err39);
            }
            errors++;
          }
          if (data.additionalRootCount === void 0) {
            const err40 = { instancePath, schemaPath: "#/oneOf/4/required", keyword: "required", params: { missingProperty: "additionalRootCount" }, message: "must have required property 'additionalRootCount'" };
            if (vErrors === null) {
              vErrors = [err40];
            } else {
              vErrors.push(err40);
            }
            errors++;
          }
          if (data.activationPathCount === void 0) {
            const err41 = { instancePath, schemaPath: "#/oneOf/4/required", keyword: "required", params: { missingProperty: "activationPathCount" }, message: "must have required property 'activationPathCount'" };
            if (vErrors === null) {
              vErrors = [err41];
            } else {
              vErrors.push(err41);
            }
            errors++;
          }
          if (data.activationPathCount !== void 0) {
            let data15 = data.activationPathCount;
            if (!(typeof data15 == "number" && (!(data15 % 1) && !isNaN(data15)))) {
              const err42 = { instancePath: instancePath + "/activationPathCount", schemaPath: "#/oneOf/4/properties/activationPathCount/type", keyword: "type", params: { type: "integer" }, message: "must be integer" };
              if (vErrors === null) {
                vErrors = [err42];
              } else {
                vErrors.push(err42);
              }
              errors++;
            }
            if (typeof data15 == "number") {
              if (data15 < 0 || isNaN(data15)) {
                const err43 = { instancePath: instancePath + "/activationPathCount", schemaPath: "#/oneOf/4/properties/activationPathCount/minimum", keyword: "minimum", params: { comparison: ">=", limit: 0 }, message: "must be >= 0" };
                if (vErrors === null) {
                  vErrors = [err43];
                } else {
                  vErrors.push(err43);
                }
                errors++;
              }
              if (!formats4.validate(data15)) {
                const err44 = { instancePath: instancePath + "/activationPathCount", schemaPath: "#/oneOf/4/properties/activationPathCount/format", keyword: "format", params: { format: "uint32" }, message: 'must match format "uint32"' };
                if (vErrors === null) {
                  vErrors = [err44];
                } else {
                  vErrors.push(err44);
                }
                errors++;
              }
            }
          }
          if (data.activationRootCount !== void 0) {
            let data16 = data.activationRootCount;
            if (!(typeof data16 == "number" && (!(data16 % 1) && !isNaN(data16)))) {
              const err45 = { instancePath: instancePath + "/activationRootCount", schemaPath: "#/oneOf/4/properties/activationRootCount/type", keyword: "type", params: { type: "integer" }, message: "must be integer" };
              if (vErrors === null) {
                vErrors = [err45];
              } else {
                vErrors.push(err45);
              }
              errors++;
            }
            if (typeof data16 == "number") {
              if (data16 < 0 || isNaN(data16)) {
                const err46 = { instancePath: instancePath + "/activationRootCount", schemaPath: "#/oneOf/4/properties/activationRootCount/minimum", keyword: "minimum", params: { comparison: ">=", limit: 0 }, message: "must be >= 0" };
                if (vErrors === null) {
                  vErrors = [err46];
                } else {
                  vErrors.push(err46);
                }
                errors++;
              }
              if (!formats4.validate(data16)) {
                const err47 = { instancePath: instancePath + "/activationRootCount", schemaPath: "#/oneOf/4/properties/activationRootCount/format", keyword: "format", params: { format: "uint32" }, message: 'must match format "uint32"' };
                if (vErrors === null) {
                  vErrors = [err47];
                } else {
                  vErrors.push(err47);
                }
                errors++;
              }
            }
          }
          if (data.additionalRootCount !== void 0) {
            let data17 = data.additionalRootCount;
            if (!(typeof data17 == "number" && (!(data17 % 1) && !isNaN(data17)))) {
              const err48 = { instancePath: instancePath + "/additionalRootCount", schemaPath: "#/oneOf/4/properties/additionalRootCount/type", keyword: "type", params: { type: "integer" }, message: "must be integer" };
              if (vErrors === null) {
                vErrors = [err48];
              } else {
                vErrors.push(err48);
              }
              errors++;
            }
            if (typeof data17 == "number") {
              if (data17 < 0 || isNaN(data17)) {
                const err49 = { instancePath: instancePath + "/additionalRootCount", schemaPath: "#/oneOf/4/properties/additionalRootCount/minimum", keyword: "minimum", params: { comparison: ">=", limit: 0 }, message: "must be >= 0" };
                if (vErrors === null) {
                  vErrors = [err49];
                } else {
                  vErrors.push(err49);
                }
                errors++;
              }
              if (!formats4.validate(data17)) {
                const err50 = { instancePath: instancePath + "/additionalRootCount", schemaPath: "#/oneOf/4/properties/additionalRootCount/format", keyword: "format", params: { format: "uint32" }, message: 'must match format "uint32"' };
                if (vErrors === null) {
                  vErrors = [err50];
                } else {
                  vErrors.push(err50);
                }
                errors++;
              }
            }
          }
          if (data.executionWorkspaceId !== void 0) {
            if (typeof data.executionWorkspaceId !== "string") {
              const err51 = { instancePath: instancePath + "/executionWorkspaceId", schemaPath: "#/oneOf/4/properties/executionWorkspaceId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
              if (vErrors === null) {
                vErrors = [err51];
              } else {
                vErrors.push(err51);
              }
              errors++;
            }
          }
          if (data.kind !== void 0) {
            let data19 = data.kind;
            if (typeof data19 !== "string") {
              const err52 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/4/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
              if (vErrors === null) {
                vErrors = [err52];
              } else {
                vErrors.push(err52);
              }
              errors++;
            }
            if ("session" !== data19) {
              const err53 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/4/properties/kind/const", keyword: "const", params: { allowedValue: "session" }, message: "must be equal to constant" };
              if (vErrors === null) {
                vErrors = [err53];
              } else {
                vErrors.push(err53);
              }
              errors++;
            }
          }
        } else {
          const err54 = { instancePath, schemaPath: "#/oneOf/4/type", keyword: "type", params: { type: "object" }, message: "must be object" };
          if (vErrors === null) {
            vErrors = [err54];
          } else {
            vErrors.push(err54);
          }
          errors++;
        }
        var _valid0 = _errs38 === errors;
        if (_valid0 && valid0) {
          valid0 = false;
          passing0 = [passing0, 4];
        } else {
          if (_valid0) {
            valid0 = true;
            passing0 = 4;
            if (props0 !== true) {
              props0 = props0 || {};
              props0.activationPathCount = true;
              props0.activationRootCount = true;
              props0.additionalRootCount = true;
              props0.executionWorkspaceId = true;
              props0.kind = true;
            }
          }
          const _errs50 = errors;
          if (data && typeof data == "object" && !Array.isArray(data)) {
            if (data.kind === void 0) {
              const err55 = { instancePath, schemaPath: "#/oneOf/5/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
              if (vErrors === null) {
                vErrors = [err55];
              } else {
                vErrors.push(err55);
              }
              errors++;
            }
            if (data.lifecycleId === void 0) {
              const err56 = { instancePath, schemaPath: "#/oneOf/5/required", keyword: "required", params: { missingProperty: "lifecycleId" }, message: "must have required property 'lifecycleId'" };
              if (vErrors === null) {
                vErrors = [err56];
              } else {
                vErrors.push(err56);
              }
              errors++;
            }
            if (data.eventId === void 0) {
              const err57 = { instancePath, schemaPath: "#/oneOf/5/required", keyword: "required", params: { missingProperty: "eventId" }, message: "must have required property 'eventId'" };
              if (vErrors === null) {
                vErrors = [err57];
              } else {
                vErrors.push(err57);
              }
              errors++;
            }
            if (data.actionId === void 0) {
              const err58 = { instancePath, schemaPath: "#/oneOf/5/required", keyword: "required", params: { missingProperty: "actionId" }, message: "must have required property 'actionId'" };
              if (vErrors === null) {
                vErrors = [err58];
              } else {
                vErrors.push(err58);
              }
              errors++;
            }
            if (data.actionId !== void 0) {
              if (typeof data.actionId !== "string") {
                const err59 = { instancePath: instancePath + "/actionId", schemaPath: "#/oneOf/5/properties/actionId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                if (vErrors === null) {
                  vErrors = [err59];
                } else {
                  vErrors.push(err59);
                }
                errors++;
              }
            }
            if (data.eventId !== void 0) {
              if (typeof data.eventId !== "string") {
                const err60 = { instancePath: instancePath + "/eventId", schemaPath: "#/oneOf/5/properties/eventId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                if (vErrors === null) {
                  vErrors = [err60];
                } else {
                  vErrors.push(err60);
                }
                errors++;
              }
            }
            if (data.kind !== void 0) {
              let data22 = data.kind;
              if (typeof data22 !== "string") {
                const err61 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/5/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                if (vErrors === null) {
                  vErrors = [err61];
                } else {
                  vErrors.push(err61);
                }
                errors++;
              }
              if ("lifecycle" !== data22) {
                const err62 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/5/properties/kind/const", keyword: "const", params: { allowedValue: "lifecycle" }, message: "must be equal to constant" };
                if (vErrors === null) {
                  vErrors = [err62];
                } else {
                  vErrors.push(err62);
                }
                errors++;
              }
            }
            if (data.lifecycleId !== void 0) {
              if (typeof data.lifecycleId !== "string") {
                const err63 = { instancePath: instancePath + "/lifecycleId", schemaPath: "#/oneOf/5/properties/lifecycleId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                if (vErrors === null) {
                  vErrors = [err63];
                } else {
                  vErrors.push(err63);
                }
                errors++;
              }
            }
          } else {
            const err64 = { instancePath, schemaPath: "#/oneOf/5/type", keyword: "type", params: { type: "object" }, message: "must be object" };
            if (vErrors === null) {
              vErrors = [err64];
            } else {
              vErrors.push(err64);
            }
            errors++;
          }
          var _valid0 = _errs50 === errors;
          if (_valid0 && valid0) {
            valid0 = false;
            passing0 = [passing0, 5];
          } else {
            if (_valid0) {
              valid0 = true;
              passing0 = 5;
              if (props0 !== true) {
                props0 = props0 || {};
                props0.actionId = true;
                props0.eventId = true;
                props0.kind = true;
                props0.lifecycleId = true;
              }
            }
            const _errs60 = errors;
            if (data && typeof data == "object" && !Array.isArray(data)) {
              if (data.kind === void 0) {
                const err65 = { instancePath, schemaPath: "#/oneOf/6/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
                if (vErrors === null) {
                  vErrors = [err65];
                } else {
                  vErrors.push(err65);
                }
                errors++;
              }
              if (data.authorityRootId === void 0) {
                const err66 = { instancePath, schemaPath: "#/oneOf/6/required", keyword: "required", params: { missingProperty: "authorityRootId" }, message: "must have required property 'authorityRootId'" };
                if (vErrors === null) {
                  vErrors = [err66];
                } else {
                  vErrors.push(err66);
                }
                errors++;
              }
              if (data.authorityRootId !== void 0) {
                if (typeof data.authorityRootId !== "string") {
                  const err67 = { instancePath: instancePath + "/authorityRootId", schemaPath: "#/oneOf/6/properties/authorityRootId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                  if (vErrors === null) {
                    vErrors = [err67];
                  } else {
                    vErrors.push(err67);
                  }
                  errors++;
                }
              }
              if (data.kind !== void 0) {
                let data25 = data.kind;
                if (typeof data25 !== "string") {
                  const err68 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/6/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                  if (vErrors === null) {
                    vErrors = [err68];
                  } else {
                    vErrors.push(err68);
                  }
                  errors++;
                }
                if ("authority-root" !== data25) {
                  const err69 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/6/properties/kind/const", keyword: "const", params: { allowedValue: "authority-root" }, message: "must be equal to constant" };
                  if (vErrors === null) {
                    vErrors = [err69];
                  } else {
                    vErrors.push(err69);
                  }
                  errors++;
                }
              }
            } else {
              const err70 = { instancePath, schemaPath: "#/oneOf/6/type", keyword: "type", params: { type: "object" }, message: "must be object" };
              if (vErrors === null) {
                vErrors = [err70];
              } else {
                vErrors.push(err70);
              }
              errors++;
            }
            var _valid0 = _errs60 === errors;
            if (_valid0 && valid0) {
              valid0 = false;
              passing0 = [passing0, 6];
            } else {
              if (_valid0) {
                valid0 = true;
                passing0 = 6;
                if (props0 !== true) {
                  props0 = props0 || {};
                  props0.authorityRootId = true;
                  props0.kind = true;
                }
              }
              const _errs66 = errors;
              if (data && typeof data == "object" && !Array.isArray(data)) {
                if (data.kind === void 0) {
                  const err71 = { instancePath, schemaPath: "#/oneOf/7/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
                  if (vErrors === null) {
                    vErrors = [err71];
                  } else {
                    vErrors.push(err71);
                  }
                  errors++;
                }
                if (data.authorityCoordinate === void 0) {
                  const err72 = { instancePath, schemaPath: "#/oneOf/7/required", keyword: "required", params: { missingProperty: "authorityCoordinate" }, message: "must have required property 'authorityCoordinate'" };
                  if (vErrors === null) {
                    vErrors = [err72];
                  } else {
                    vErrors.push(err72);
                  }
                  errors++;
                }
                if (data.authorityCoordinate !== void 0) {
                  if (typeof data.authorityCoordinate !== "string") {
                    const err73 = { instancePath: instancePath + "/authorityCoordinate", schemaPath: "#/oneOf/7/properties/authorityCoordinate/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                    if (vErrors === null) {
                      vErrors = [err73];
                    } else {
                      vErrors.push(err73);
                    }
                    errors++;
                  }
                }
                if (data.kind !== void 0) {
                  let data27 = data.kind;
                  if (typeof data27 !== "string") {
                    const err74 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/7/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                    if (vErrors === null) {
                      vErrors = [err74];
                    } else {
                      vErrors.push(err74);
                    }
                    errors++;
                  }
                  if ("disclosure-ceiling" !== data27) {
                    const err75 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/7/properties/kind/const", keyword: "const", params: { allowedValue: "disclosure-ceiling" }, message: "must be equal to constant" };
                    if (vErrors === null) {
                      vErrors = [err75];
                    } else {
                      vErrors.push(err75);
                    }
                    errors++;
                  }
                }
              } else {
                const err76 = { instancePath, schemaPath: "#/oneOf/7/type", keyword: "type", params: { type: "object" }, message: "must be object" };
                if (vErrors === null) {
                  vErrors = [err76];
                } else {
                  vErrors.push(err76);
                }
                errors++;
              }
              var _valid0 = _errs66 === errors;
              if (_valid0 && valid0) {
                valid0 = false;
                passing0 = [passing0, 7];
              } else {
                if (_valid0) {
                  valid0 = true;
                  passing0 = 7;
                  if (props0 !== true) {
                    props0 = props0 || {};
                    props0.authorityCoordinate = true;
                    props0.kind = true;
                  }
                }
              }
            }
          }
        }
      }
    }
  }
  if (!valid0) {
    const err77 = { instancePath, schemaPath: "#/oneOf", keyword: "oneOf", params: { passingSchemas: passing0 }, message: "must match exactly one schema in oneOf" };
    if (vErrors === null) {
      vErrors = [err77];
    } else {
      vErrors.push(err77);
    }
    errors++;
  } else {
    errors = _errs0;
    if (vErrors !== null) {
      if (_errs0) {
        vErrors.length = _errs0;
      } else {
        vErrors = null;
      }
    }
  }
  validate159.errors = vErrors;
  evaluated0.props = props0;
  return errors === 0;
}
validate159.evaluated = { "dynamicProps": true, "dynamicItems": false };
function validate163(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate163.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  const _errs0 = errors;
  let valid0 = false;
  let passing0 = null;
  const _errs1 = errors;
  if (typeof data !== "string") {
    const err0 = { instancePath, schemaPath: "#/oneOf/0/type", keyword: "type", params: { type: "string" }, message: "must be string" };
    if (vErrors === null) {
      vErrors = [err0];
    } else {
      vErrors.push(err0);
    }
    errors++;
  }
  if ("unbound" !== data) {
    const err1 = { instancePath, schemaPath: "#/oneOf/0/const", keyword: "const", params: { allowedValue: "unbound" }, message: "must be equal to constant" };
    if (vErrors === null) {
      vErrors = [err1];
    } else {
      vErrors.push(err1);
    }
    errors++;
  }
  var _valid0 = _errs1 === errors;
  if (_valid0) {
    valid0 = true;
    passing0 = 0;
  }
  const _errs3 = errors;
  if (typeof data !== "string") {
    const err2 = { instancePath, schemaPath: "#/oneOf/1/type", keyword: "type", params: { type: "string" }, message: "must be string" };
    if (vErrors === null) {
      vErrors = [err2];
    } else {
      vErrors.push(err2);
    }
    errors++;
  }
  if ("semantic-only" !== data) {
    const err3 = { instancePath, schemaPath: "#/oneOf/1/const", keyword: "const", params: { allowedValue: "semantic-only" }, message: "must be equal to constant" };
    if (vErrors === null) {
      vErrors = [err3];
    } else {
      vErrors.push(err3);
    }
    errors++;
  }
  var _valid0 = _errs3 === errors;
  if (_valid0 && valid0) {
    valid0 = false;
    passing0 = [passing0, 1];
  } else {
    if (_valid0) {
      valid0 = true;
      passing0 = 1;
    }
    const _errs5 = errors;
    if (typeof data !== "string") {
      const err4 = { instancePath, schemaPath: "#/oneOf/2/type", keyword: "type", params: { type: "string" }, message: "must be string" };
      if (vErrors === null) {
        vErrors = [err4];
      } else {
        vErrors.push(err4);
      }
      errors++;
    }
    if ("capability-bearing" !== data) {
      const err5 = { instancePath, schemaPath: "#/oneOf/2/const", keyword: "const", params: { allowedValue: "capability-bearing" }, message: "must be equal to constant" };
      if (vErrors === null) {
        vErrors = [err5];
      } else {
        vErrors.push(err5);
      }
      errors++;
    }
    var _valid0 = _errs5 === errors;
    if (_valid0 && valid0) {
      valid0 = false;
      passing0 = [passing0, 2];
    } else {
      if (_valid0) {
        valid0 = true;
        passing0 = 2;
      }
    }
  }
  if (!valid0) {
    const err6 = { instancePath, schemaPath: "#/oneOf", keyword: "oneOf", params: { passingSchemas: passing0 }, message: "must match exactly one schema in oneOf" };
    if (vErrors === null) {
      vErrors = [err6];
    } else {
      vErrors.push(err6);
    }
    errors++;
  } else {
    errors = _errs0;
    if (vErrors !== null) {
      if (_errs0) {
        vErrors.length = _errs0;
      } else {
        vErrors = null;
      }
    }
  }
  validate163.errors = vErrors;
  return errors === 0;
}
validate163.evaluated = { "dynamicProps": false, "dynamicItems": false };
function validate165(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate165.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  const _errs0 = errors;
  let valid0 = false;
  let passing0 = null;
  const _errs1 = errors;
  if (typeof data !== "string") {
    const err0 = { instancePath, schemaPath: "#/oneOf/0/type", keyword: "type", params: { type: "string" }, message: "must be string" };
    if (vErrors === null) {
      vErrors = [err0];
    } else {
      vErrors.push(err0);
    }
    errors++;
  }
  if ("unknown" !== data) {
    const err1 = { instancePath, schemaPath: "#/oneOf/0/const", keyword: "const", params: { allowedValue: "unknown" }, message: "must be equal to constant" };
    if (vErrors === null) {
      vErrors = [err1];
    } else {
      vErrors.push(err1);
    }
    errors++;
  }
  var _valid0 = _errs1 === errors;
  if (_valid0) {
    valid0 = true;
    passing0 = 0;
  }
  if (!valid0) {
    const err2 = { instancePath, schemaPath: "#/oneOf", keyword: "oneOf", params: { passingSchemas: passing0 }, message: "must match exactly one schema in oneOf" };
    if (vErrors === null) {
      vErrors = [err2];
    } else {
      vErrors.push(err2);
    }
    errors++;
  } else {
    errors = _errs0;
    if (vErrors !== null) {
      if (_errs0) {
        vErrors.length = _errs0;
      } else {
        vErrors = null;
      }
    }
  }
  validate165.errors = vErrors;
  return errors === 0;
}
validate165.evaluated = { "dynamicProps": false, "dynamicItems": false };
function validate169(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate169.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  const _errs0 = errors;
  let valid0 = false;
  let passing0 = null;
  const _errs1 = errors;
  if (typeof data !== "string") {
    const err0 = { instancePath, schemaPath: "#/oneOf/0/type", keyword: "type", params: { type: "string" }, message: "must be string" };
    if (vErrors === null) {
      vErrors = [err0];
    } else {
      vErrors.push(err0);
    }
    errors++;
  }
  if ("host-native" !== data) {
    const err1 = { instancePath, schemaPath: "#/oneOf/0/const", keyword: "const", params: { allowedValue: "host-native" }, message: "must be equal to constant" };
    if (vErrors === null) {
      vErrors = [err1];
    } else {
      vErrors.push(err1);
    }
    errors++;
  }
  var _valid0 = _errs1 === errors;
  if (_valid0) {
    valid0 = true;
    passing0 = 0;
  }
  const _errs3 = errors;
  if (typeof data !== "string") {
    const err2 = { instancePath, schemaPath: "#/oneOf/1/type", keyword: "type", params: { type: "string" }, message: "must be string" };
    if (vErrors === null) {
      vErrors = [err2];
    } else {
      vErrors.push(err2);
    }
    errors++;
  }
  if ("native-rust" !== data) {
    const err3 = { instancePath, schemaPath: "#/oneOf/1/const", keyword: "const", params: { allowedValue: "native-rust" }, message: "must be equal to constant" };
    if (vErrors === null) {
      vErrors = [err3];
    } else {
      vErrors.push(err3);
    }
    errors++;
  }
  var _valid0 = _errs3 === errors;
  if (_valid0 && valid0) {
    valid0 = false;
    passing0 = [passing0, 1];
  } else {
    if (_valid0) {
      valid0 = true;
      passing0 = 1;
    }
    const _errs5 = errors;
    if (typeof data !== "string") {
      const err4 = { instancePath, schemaPath: "#/oneOf/2/type", keyword: "type", params: { type: "string" }, message: "must be string" };
      if (vErrors === null) {
        vErrors = [err4];
      } else {
        vErrors.push(err4);
      }
      errors++;
    }
    if ("connector" !== data) {
      const err5 = { instancePath, schemaPath: "#/oneOf/2/const", keyword: "const", params: { allowedValue: "connector" }, message: "must be equal to constant" };
      if (vErrors === null) {
        vErrors = [err5];
      } else {
        vErrors.push(err5);
      }
      errors++;
    }
    var _valid0 = _errs5 === errors;
    if (_valid0 && valid0) {
      valid0 = false;
      passing0 = [passing0, 2];
    } else {
      if (_valid0) {
        valid0 = true;
        passing0 = 2;
      }
    }
  }
  if (!valid0) {
    const err6 = { instancePath, schemaPath: "#/oneOf", keyword: "oneOf", params: { passingSchemas: passing0 }, message: "must match exactly one schema in oneOf" };
    if (vErrors === null) {
      vErrors = [err6];
    } else {
      vErrors.push(err6);
    }
    errors++;
  } else {
    errors = _errs0;
    if (vErrors !== null) {
      if (_errs0) {
        vErrors.length = _errs0;
      } else {
        vErrors = null;
      }
    }
  }
  validate169.errors = vErrors;
  return errors === 0;
}
validate169.evaluated = { "dynamicProps": false, "dynamicItems": false };
function validate168(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate168.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  const _errs0 = errors;
  let valid0 = false;
  let passing0 = null;
  const _errs1 = errors;
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.kind === void 0) {
      const err0 = { instancePath, schemaPath: "#/oneOf/0/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
      if (vErrors === null) {
        vErrors = [err0];
      } else {
        vErrors.push(err0);
      }
      errors++;
    }
    if (data.provider === void 0) {
      const err1 = { instancePath, schemaPath: "#/oneOf/0/required", keyword: "required", params: { missingProperty: "provider" }, message: "must have required property 'provider'" };
      if (vErrors === null) {
        vErrors = [err1];
      } else {
        vErrors.push(err1);
      }
      errors++;
    }
    if (data.kind !== void 0) {
      let data0 = data.kind;
      if (typeof data0 !== "string") {
        const err2 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/0/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err2];
        } else {
          vErrors.push(err2);
        }
        errors++;
      }
      if ("bound" !== data0) {
        const err3 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/0/properties/kind/const", keyword: "const", params: { allowedValue: "bound" }, message: "must be equal to constant" };
        if (vErrors === null) {
          vErrors = [err3];
        } else {
          vErrors.push(err3);
        }
        errors++;
      }
    }
    if (data.provider !== void 0) {
      if (!validate169(data.provider, { instancePath: instancePath + "/provider", parentData: data, parentDataProperty: "provider", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate169.errors : vErrors.concat(validate169.errors);
        errors = vErrors.length;
      }
    }
  } else {
    const err4 = { instancePath, schemaPath: "#/oneOf/0/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err4];
    } else {
      vErrors.push(err4);
    }
    errors++;
  }
  var _valid0 = _errs1 === errors;
  if (_valid0) {
    valid0 = true;
    passing0 = 0;
    var props0 = {};
    props0.kind = true;
    props0.provider = true;
  }
  const _errs6 = errors;
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.kind === void 0) {
      const err5 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
      if (vErrors === null) {
        vErrors = [err5];
      } else {
        vErrors.push(err5);
      }
      errors++;
    }
    if (data.kind !== void 0) {
      let data2 = data.kind;
      if (typeof data2 !== "string") {
        const err6 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/1/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err6];
        } else {
          vErrors.push(err6);
        }
        errors++;
      }
      if ("missing-binding" !== data2) {
        const err7 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/1/properties/kind/const", keyword: "const", params: { allowedValue: "missing-binding" }, message: "must be equal to constant" };
        if (vErrors === null) {
          vErrors = [err7];
        } else {
          vErrors.push(err7);
        }
        errors++;
      }
    }
  } else {
    const err8 = { instancePath, schemaPath: "#/oneOf/1/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err8];
    } else {
      vErrors.push(err8);
    }
    errors++;
  }
  var _valid0 = _errs6 === errors;
  if (_valid0 && valid0) {
    valid0 = false;
    passing0 = [passing0, 1];
  } else {
    if (_valid0) {
      valid0 = true;
      passing0 = 1;
      if (props0 !== true) {
        props0 = props0 || {};
        props0.kind = true;
      }
    }
    const _errs10 = errors;
    if (data && typeof data == "object" && !Array.isArray(data)) {
      if (data.kind === void 0) {
        const err9 = { instancePath, schemaPath: "#/oneOf/2/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
        if (vErrors === null) {
          vErrors = [err9];
        } else {
          vErrors.push(err9);
        }
        errors++;
      }
      if (data.kind !== void 0) {
        let data3 = data.kind;
        if (typeof data3 !== "string") {
          const err10 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/2/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
          if (vErrors === null) {
            vErrors = [err10];
          } else {
            vErrors.push(err10);
          }
          errors++;
        }
        if ("excluded-by-narrowing" !== data3) {
          const err11 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/2/properties/kind/const", keyword: "const", params: { allowedValue: "excluded-by-narrowing" }, message: "must be equal to constant" };
          if (vErrors === null) {
            vErrors = [err11];
          } else {
            vErrors.push(err11);
          }
          errors++;
        }
      }
    } else {
      const err12 = { instancePath, schemaPath: "#/oneOf/2/type", keyword: "type", params: { type: "object" }, message: "must be object" };
      if (vErrors === null) {
        vErrors = [err12];
      } else {
        vErrors.push(err12);
      }
      errors++;
    }
    var _valid0 = _errs10 === errors;
    if (_valid0 && valid0) {
      valid0 = false;
      passing0 = [passing0, 2];
    } else {
      if (_valid0) {
        valid0 = true;
        passing0 = 2;
        if (props0 !== true) {
          props0 = props0 || {};
          props0.kind = true;
        }
      }
      const _errs14 = errors;
      if (data && typeof data == "object" && !Array.isArray(data)) {
        if (data.kind === void 0) {
          const err13 = { instancePath, schemaPath: "#/oneOf/3/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
          if (vErrors === null) {
            vErrors = [err13];
          } else {
            vErrors.push(err13);
          }
          errors++;
        }
        if (data.kind !== void 0) {
          let data4 = data.kind;
          if (typeof data4 !== "string") {
            const err14 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/3/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
            if (vErrors === null) {
              vErrors = [err14];
            } else {
              vErrors.push(err14);
            }
            errors++;
          }
          if ("effect-mismatch" !== data4) {
            const err15 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/3/properties/kind/const", keyword: "const", params: { allowedValue: "effect-mismatch" }, message: "must be equal to constant" };
            if (vErrors === null) {
              vErrors = [err15];
            } else {
              vErrors.push(err15);
            }
            errors++;
          }
        }
      } else {
        const err16 = { instancePath, schemaPath: "#/oneOf/3/type", keyword: "type", params: { type: "object" }, message: "must be object" };
        if (vErrors === null) {
          vErrors = [err16];
        } else {
          vErrors.push(err16);
        }
        errors++;
      }
      var _valid0 = _errs14 === errors;
      if (_valid0 && valid0) {
        valid0 = false;
        passing0 = [passing0, 3];
      } else {
        if (_valid0) {
          valid0 = true;
          passing0 = 3;
          if (props0 !== true) {
            props0 = props0 || {};
            props0.kind = true;
          }
        }
        const _errs18 = errors;
        if (data && typeof data == "object" && !Array.isArray(data)) {
          if (data.kind === void 0) {
            const err17 = { instancePath, schemaPath: "#/oneOf/4/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
            if (vErrors === null) {
              vErrors = [err17];
            } else {
              vErrors.push(err17);
            }
            errors++;
          }
          if (data.kind !== void 0) {
            let data5 = data.kind;
            if (typeof data5 !== "string") {
              const err18 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/4/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
              if (vErrors === null) {
                vErrors = [err18];
              } else {
                vErrors.push(err18);
              }
              errors++;
            }
            if ("recipe-contract-mismatch" !== data5) {
              const err19 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/4/properties/kind/const", keyword: "const", params: { allowedValue: "recipe-contract-mismatch" }, message: "must be equal to constant" };
              if (vErrors === null) {
                vErrors = [err19];
              } else {
                vErrors.push(err19);
              }
              errors++;
            }
          }
        } else {
          const err20 = { instancePath, schemaPath: "#/oneOf/4/type", keyword: "type", params: { type: "object" }, message: "must be object" };
          if (vErrors === null) {
            vErrors = [err20];
          } else {
            vErrors.push(err20);
          }
          errors++;
        }
        var _valid0 = _errs18 === errors;
        if (_valid0 && valid0) {
          valid0 = false;
          passing0 = [passing0, 4];
        } else {
          if (_valid0) {
            valid0 = true;
            passing0 = 4;
            if (props0 !== true) {
              props0 = props0 || {};
              props0.kind = true;
            }
          }
        }
      }
    }
  }
  if (!valid0) {
    const err21 = { instancePath, schemaPath: "#/oneOf", keyword: "oneOf", params: { passingSchemas: passing0 }, message: "must match exactly one schema in oneOf" };
    if (vErrors === null) {
      vErrors = [err21];
    } else {
      vErrors.push(err21);
    }
    errors++;
  } else {
    errors = _errs0;
    if (vErrors !== null) {
      if (_errs0) {
        vErrors.length = _errs0;
      } else {
        vErrors = null;
      }
    }
  }
  validate168.errors = vErrors;
  evaluated0.props = props0;
  return errors === 0;
}
validate168.evaluated = { "dynamicProps": true, "dynamicItems": false };
function validate173(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate173.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  const _errs0 = errors;
  let valid0 = false;
  let passing0 = null;
  const _errs1 = errors;
  if (typeof data !== "string") {
    const err0 = { instancePath, schemaPath: "#/oneOf/0/type", keyword: "type", params: { type: "string" }, message: "must be string" };
    if (vErrors === null) {
      vErrors = [err0];
    } else {
      vErrors.push(err0);
    }
    errors++;
  }
  if ("capability-not-permitted" !== data) {
    const err1 = { instancePath, schemaPath: "#/oneOf/0/const", keyword: "const", params: { allowedValue: "capability-not-permitted" }, message: "must be equal to constant" };
    if (vErrors === null) {
      vErrors = [err1];
    } else {
      vErrors.push(err1);
    }
    errors++;
  }
  var _valid0 = _errs1 === errors;
  if (_valid0) {
    valid0 = true;
    passing0 = 0;
  }
  const _errs3 = errors;
  if (typeof data !== "string") {
    const err2 = { instancePath, schemaPath: "#/oneOf/1/type", keyword: "type", params: { type: "string" }, message: "must be string" };
    if (vErrors === null) {
      vErrors = [err2];
    } else {
      vErrors.push(err2);
    }
    errors++;
  }
  if ("effect-not-permitted" !== data) {
    const err3 = { instancePath, schemaPath: "#/oneOf/1/const", keyword: "const", params: { allowedValue: "effect-not-permitted" }, message: "must be equal to constant" };
    if (vErrors === null) {
      vErrors = [err3];
    } else {
      vErrors.push(err3);
    }
    errors++;
  }
  var _valid0 = _errs3 === errors;
  if (_valid0 && valid0) {
    valid0 = false;
    passing0 = [passing0, 1];
  } else {
    if (_valid0) {
      valid0 = true;
      passing0 = 1;
    }
    const _errs5 = errors;
    if (typeof data !== "string") {
      const err4 = { instancePath, schemaPath: "#/oneOf/2/type", keyword: "type", params: { type: "string" }, message: "must be string" };
      if (vErrors === null) {
        vErrors = [err4];
      } else {
        vErrors.push(err4);
      }
      errors++;
    }
    if ("capability-and-effect-not-permitted" !== data) {
      const err5 = { instancePath, schemaPath: "#/oneOf/2/const", keyword: "const", params: { allowedValue: "capability-and-effect-not-permitted" }, message: "must be equal to constant" };
      if (vErrors === null) {
        vErrors = [err5];
      } else {
        vErrors.push(err5);
      }
      errors++;
    }
    var _valid0 = _errs5 === errors;
    if (_valid0 && valid0) {
      valid0 = false;
      passing0 = [passing0, 2];
    } else {
      if (_valid0) {
        valid0 = true;
        passing0 = 2;
      }
    }
  }
  if (!valid0) {
    const err6 = { instancePath, schemaPath: "#/oneOf", keyword: "oneOf", params: { passingSchemas: passing0 }, message: "must match exactly one schema in oneOf" };
    if (vErrors === null) {
      vErrors = [err6];
    } else {
      vErrors.push(err6);
    }
    errors++;
  } else {
    errors = _errs0;
    if (vErrors !== null) {
      if (_errs0) {
        vErrors.length = _errs0;
      } else {
        vErrors = null;
      }
    }
  }
  validate173.errors = vErrors;
  return errors === 0;
}
validate173.evaluated = { "dynamicProps": false, "dynamicItems": false };
function validate175(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate175.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  const _errs0 = errors;
  let valid0 = false;
  let passing0 = null;
  const _errs1 = errors;
  if (typeof data !== "string") {
    const err0 = { instancePath, schemaPath: "#/oneOf/0/type", keyword: "type", params: { type: "string" }, message: "must be string" };
    if (vErrors === null) {
      vErrors = [err0];
    } else {
      vErrors.push(err0);
    }
    errors++;
  }
  if ("capability-forbidden" !== data) {
    const err1 = { instancePath, schemaPath: "#/oneOf/0/const", keyword: "const", params: { allowedValue: "capability-forbidden" }, message: "must be equal to constant" };
    if (vErrors === null) {
      vErrors = [err1];
    } else {
      vErrors.push(err1);
    }
    errors++;
  }
  var _valid0 = _errs1 === errors;
  if (_valid0) {
    valid0 = true;
    passing0 = 0;
  }
  const _errs3 = errors;
  if (typeof data !== "string") {
    const err2 = { instancePath, schemaPath: "#/oneOf/1/type", keyword: "type", params: { type: "string" }, message: "must be string" };
    if (vErrors === null) {
      vErrors = [err2];
    } else {
      vErrors.push(err2);
    }
    errors++;
  }
  if ("effect-forbidden" !== data) {
    const err3 = { instancePath, schemaPath: "#/oneOf/1/const", keyword: "const", params: { allowedValue: "effect-forbidden" }, message: "must be equal to constant" };
    if (vErrors === null) {
      vErrors = [err3];
    } else {
      vErrors.push(err3);
    }
    errors++;
  }
  var _valid0 = _errs3 === errors;
  if (_valid0 && valid0) {
    valid0 = false;
    passing0 = [passing0, 1];
  } else {
    if (_valid0) {
      valid0 = true;
      passing0 = 1;
    }
    const _errs5 = errors;
    if (typeof data !== "string") {
      const err4 = { instancePath, schemaPath: "#/oneOf/2/type", keyword: "type", params: { type: "string" }, message: "must be string" };
      if (vErrors === null) {
        vErrors = [err4];
      } else {
        vErrors.push(err4);
      }
      errors++;
    }
    if ("capability-and-effect-forbidden" !== data) {
      const err5 = { instancePath, schemaPath: "#/oneOf/2/const", keyword: "const", params: { allowedValue: "capability-and-effect-forbidden" }, message: "must be equal to constant" };
      if (vErrors === null) {
        vErrors = [err5];
      } else {
        vErrors.push(err5);
      }
      errors++;
    }
    var _valid0 = _errs5 === errors;
    if (_valid0 && valid0) {
      valid0 = false;
      passing0 = [passing0, 2];
    } else {
      if (_valid0) {
        valid0 = true;
        passing0 = 2;
      }
      const _errs7 = errors;
      if (typeof data !== "string") {
        const err6 = { instancePath, schemaPath: "#/oneOf/3/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err6];
        } else {
          vErrors.push(err6);
        }
        errors++;
      }
      if ("provider-effect-mismatch" !== data) {
        const err7 = { instancePath, schemaPath: "#/oneOf/3/const", keyword: "const", params: { allowedValue: "provider-effect-mismatch" }, message: "must be equal to constant" };
        if (vErrors === null) {
          vErrors = [err7];
        } else {
          vErrors.push(err7);
        }
        errors++;
      }
      var _valid0 = _errs7 === errors;
      if (_valid0 && valid0) {
        valid0 = false;
        passing0 = [passing0, 3];
      } else {
        if (_valid0) {
          valid0 = true;
          passing0 = 3;
        }
        const _errs9 = errors;
        if (typeof data !== "string") {
          const err8 = { instancePath, schemaPath: "#/oneOf/4/type", keyword: "type", params: { type: "string" }, message: "must be string" };
          if (vErrors === null) {
            vErrors = [err8];
          } else {
            vErrors.push(err8);
          }
          errors++;
        }
        if ("recipe-contract-mismatch" !== data) {
          const err9 = { instancePath, schemaPath: "#/oneOf/4/const", keyword: "const", params: { allowedValue: "recipe-contract-mismatch" }, message: "must be equal to constant" };
          if (vErrors === null) {
            vErrors = [err9];
          } else {
            vErrors.push(err9);
          }
          errors++;
        }
        var _valid0 = _errs9 === errors;
        if (_valid0 && valid0) {
          valid0 = false;
          passing0 = [passing0, 4];
        } else {
          if (_valid0) {
            valid0 = true;
            passing0 = 4;
          }
        }
      }
    }
  }
  if (!valid0) {
    const err10 = { instancePath, schemaPath: "#/oneOf", keyword: "oneOf", params: { passingSchemas: passing0 }, message: "must match exactly one schema in oneOf" };
    if (vErrors === null) {
      vErrors = [err10];
    } else {
      vErrors.push(err10);
    }
    errors++;
  } else {
    errors = _errs0;
    if (vErrors !== null) {
      if (_errs0) {
        vErrors.length = _errs0;
      } else {
        vErrors = null;
      }
    }
  }
  validate175.errors = vErrors;
  return errors === 0;
}
validate175.evaluated = { "dynamicProps": false, "dynamicItems": false };
function validate172(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate172.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  const _errs0 = errors;
  let valid0 = false;
  let passing0 = null;
  const _errs1 = errors;
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.kind === void 0) {
      const err0 = { instancePath, schemaPath: "#/oneOf/0/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
      if (vErrors === null) {
        vErrors = [err0];
      } else {
        vErrors.push(err0);
      }
      errors++;
    }
    if (data.kind !== void 0) {
      let data0 = data.kind;
      if (typeof data0 !== "string") {
        const err1 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/0/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err1];
        } else {
          vErrors.push(err1);
        }
        errors++;
      }
      if ("satisfied" !== data0) {
        const err2 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/0/properties/kind/const", keyword: "const", params: { allowedValue: "satisfied" }, message: "must be equal to constant" };
        if (vErrors === null) {
          vErrors = [err2];
        } else {
          vErrors.push(err2);
        }
        errors++;
      }
    }
  } else {
    const err3 = { instancePath, schemaPath: "#/oneOf/0/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err3];
    } else {
      vErrors.push(err3);
    }
    errors++;
  }
  var _valid0 = _errs1 === errors;
  if (_valid0) {
    valid0 = true;
    passing0 = 0;
    var props0 = {};
    props0.kind = true;
  }
  const _errs5 = errors;
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.kind === void 0) {
      const err4 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
      if (vErrors === null) {
        vErrors = [err4];
      } else {
        vErrors.push(err4);
      }
      errors++;
    }
    if (data.kind !== void 0) {
      let data1 = data.kind;
      if (typeof data1 !== "string") {
        const err5 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/1/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err5];
        } else {
          vErrors.push(err5);
        }
        errors++;
      }
      if ("provider-binding-absent" !== data1) {
        const err6 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/1/properties/kind/const", keyword: "const", params: { allowedValue: "provider-binding-absent" }, message: "must be equal to constant" };
        if (vErrors === null) {
          vErrors = [err6];
        } else {
          vErrors.push(err6);
        }
        errors++;
      }
    }
  } else {
    const err7 = { instancePath, schemaPath: "#/oneOf/1/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err7];
    } else {
      vErrors.push(err7);
    }
    errors++;
  }
  var _valid0 = _errs5 === errors;
  if (_valid0 && valid0) {
    valid0 = false;
    passing0 = [passing0, 1];
  } else {
    if (_valid0) {
      valid0 = true;
      passing0 = 1;
      if (props0 !== true) {
        props0 = props0 || {};
        props0.kind = true;
      }
    }
    const _errs9 = errors;
    if (data && typeof data == "object" && !Array.isArray(data)) {
      if (data.kind === void 0) {
        const err8 = { instancePath, schemaPath: "#/oneOf/2/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
        if (vErrors === null) {
          vErrors = [err8];
        } else {
          vErrors.push(err8);
        }
        errors++;
      }
      if (data.reason === void 0) {
        const err9 = { instancePath, schemaPath: "#/oneOf/2/required", keyword: "required", params: { missingProperty: "reason" }, message: "must have required property 'reason'" };
        if (vErrors === null) {
          vErrors = [err9];
        } else {
          vErrors.push(err9);
        }
        errors++;
      }
      if (data.kind !== void 0) {
        let data2 = data.kind;
        if (typeof data2 !== "string") {
          const err10 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/2/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
          if (vErrors === null) {
            vErrors = [err10];
          } else {
            vErrors.push(err10);
          }
          errors++;
        }
        if ("narrowed" !== data2) {
          const err11 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/2/properties/kind/const", keyword: "const", params: { allowedValue: "narrowed" }, message: "must be equal to constant" };
          if (vErrors === null) {
            vErrors = [err11];
          } else {
            vErrors.push(err11);
          }
          errors++;
        }
      }
      if (data.reason !== void 0) {
        if (!validate173(data.reason, { instancePath: instancePath + "/reason", parentData: data, parentDataProperty: "reason", rootData, dynamicAnchors })) {
          vErrors = vErrors === null ? validate173.errors : vErrors.concat(validate173.errors);
          errors = vErrors.length;
        }
      }
    } else {
      const err12 = { instancePath, schemaPath: "#/oneOf/2/type", keyword: "type", params: { type: "object" }, message: "must be object" };
      if (vErrors === null) {
        vErrors = [err12];
      } else {
        vErrors.push(err12);
      }
      errors++;
    }
    var _valid0 = _errs9 === errors;
    if (_valid0 && valid0) {
      valid0 = false;
      passing0 = [passing0, 2];
    } else {
      if (_valid0) {
        valid0 = true;
        passing0 = 2;
        if (props0 !== true) {
          props0 = props0 || {};
          props0.kind = true;
          props0.reason = true;
        }
      }
      const _errs14 = errors;
      if (data && typeof data == "object" && !Array.isArray(data)) {
        if (data.kind === void 0) {
          const err13 = { instancePath, schemaPath: "#/oneOf/3/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
          if (vErrors === null) {
            vErrors = [err13];
          } else {
            vErrors.push(err13);
          }
          errors++;
        }
        if (data.reason === void 0) {
          const err14 = { instancePath, schemaPath: "#/oneOf/3/required", keyword: "required", params: { missingProperty: "reason" }, message: "must have required property 'reason'" };
          if (vErrors === null) {
            vErrors = [err14];
          } else {
            vErrors.push(err14);
          }
          errors++;
        }
        if (data.kind !== void 0) {
          let data4 = data.kind;
          if (typeof data4 !== "string") {
            const err15 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/3/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
            if (vErrors === null) {
              vErrors = [err15];
            } else {
              vErrors.push(err15);
            }
            errors++;
          }
          if ("blocked" !== data4) {
            const err16 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/3/properties/kind/const", keyword: "const", params: { allowedValue: "blocked" }, message: "must be equal to constant" };
            if (vErrors === null) {
              vErrors = [err16];
            } else {
              vErrors.push(err16);
            }
            errors++;
          }
        }
        if (data.reason !== void 0) {
          if (!validate175(data.reason, { instancePath: instancePath + "/reason", parentData: data, parentDataProperty: "reason", rootData, dynamicAnchors })) {
            vErrors = vErrors === null ? validate175.errors : vErrors.concat(validate175.errors);
            errors = vErrors.length;
          }
        }
      } else {
        const err17 = { instancePath, schemaPath: "#/oneOf/3/type", keyword: "type", params: { type: "object" }, message: "must be object" };
        if (vErrors === null) {
          vErrors = [err17];
        } else {
          vErrors.push(err17);
        }
        errors++;
      }
      var _valid0 = _errs14 === errors;
      if (_valid0 && valid0) {
        valid0 = false;
        passing0 = [passing0, 3];
      } else {
        if (_valid0) {
          valid0 = true;
          passing0 = 3;
          if (props0 !== true) {
            props0 = props0 || {};
            props0.kind = true;
            props0.reason = true;
          }
        }
      }
    }
  }
  if (!valid0) {
    const err18 = { instancePath, schemaPath: "#/oneOf", keyword: "oneOf", params: { passingSchemas: passing0 }, message: "must match exactly one schema in oneOf" };
    if (vErrors === null) {
      vErrors = [err18];
    } else {
      vErrors.push(err18);
    }
    errors++;
  } else {
    errors = _errs0;
    if (vErrors !== null) {
      if (_errs0) {
        vErrors.length = _errs0;
      } else {
        vErrors = null;
      }
    }
  }
  validate172.errors = vErrors;
  evaluated0.props = props0;
  return errors === 0;
}
validate172.evaluated = { "dynamicProps": true, "dynamicItems": false };
function validate178(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate178.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  const _errs0 = errors;
  let valid0 = false;
  let passing0 = null;
  const _errs1 = errors;
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.kind === void 0) {
      const err0 = { instancePath, schemaPath: "#/oneOf/0/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
      if (vErrors === null) {
        vErrors = [err0];
      } else {
        vErrors.push(err0);
      }
      errors++;
    }
    if (data.machineId === void 0) {
      const err1 = { instancePath, schemaPath: "#/oneOf/0/required", keyword: "required", params: { missingProperty: "machineId" }, message: "must have required property 'machineId'" };
      if (vErrors === null) {
        vErrors = [err1];
      } else {
        vErrors.push(err1);
      }
      errors++;
    }
    if (data.kind !== void 0) {
      let data0 = data.kind;
      if (typeof data0 !== "string") {
        const err2 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/0/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err2];
        } else {
          vErrors.push(err2);
        }
        errors++;
      }
      if ("native-realization-unknown" !== data0) {
        const err3 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/0/properties/kind/const", keyword: "const", params: { allowedValue: "native-realization-unknown" }, message: "must be equal to constant" };
        if (vErrors === null) {
          vErrors = [err3];
        } else {
          vErrors.push(err3);
        }
        errors++;
      }
    }
    if (data.machineId !== void 0) {
      if (typeof data.machineId !== "string") {
        const err4 = { instancePath: instancePath + "/machineId", schemaPath: "#/oneOf/0/properties/machineId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err4];
        } else {
          vErrors.push(err4);
        }
        errors++;
      }
    }
  } else {
    const err5 = { instancePath, schemaPath: "#/oneOf/0/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err5];
    } else {
      vErrors.push(err5);
    }
    errors++;
  }
  var _valid0 = _errs1 === errors;
  if (_valid0) {
    valid0 = true;
    passing0 = 0;
    var props0 = {};
    props0.kind = true;
    props0.machineId = true;
  }
  const _errs7 = errors;
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.kind === void 0) {
      const err6 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
      if (vErrors === null) {
        vErrors = [err6];
      } else {
        vErrors.push(err6);
      }
      errors++;
    }
    if (data.repositoryId === void 0) {
      const err7 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "repositoryId" }, message: "must have required property 'repositoryId'" };
      if (vErrors === null) {
        vErrors = [err7];
      } else {
        vErrors.push(err7);
      }
      errors++;
    }
    if (data.operationId === void 0) {
      const err8 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "operationId" }, message: "must have required property 'operationId'" };
      if (vErrors === null) {
        vErrors = [err8];
      } else {
        vErrors.push(err8);
      }
      errors++;
    }
    if (data.kind !== void 0) {
      let data2 = data.kind;
      if (typeof data2 !== "string") {
        const err9 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/1/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err9];
        } else {
          vErrors.push(err9);
        }
        errors++;
      }
      if ("operation-unbound" !== data2) {
        const err10 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/1/properties/kind/const", keyword: "const", params: { allowedValue: "operation-unbound" }, message: "must be equal to constant" };
        if (vErrors === null) {
          vErrors = [err10];
        } else {
          vErrors.push(err10);
        }
        errors++;
      }
    }
    if (data.operationId !== void 0) {
      if (typeof data.operationId !== "string") {
        const err11 = { instancePath: instancePath + "/operationId", schemaPath: "#/oneOf/1/properties/operationId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err11];
        } else {
          vErrors.push(err11);
        }
        errors++;
      }
    }
    if (data.repositoryId !== void 0) {
      if (typeof data.repositoryId !== "string") {
        const err12 = { instancePath: instancePath + "/repositoryId", schemaPath: "#/oneOf/1/properties/repositoryId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err12];
        } else {
          vErrors.push(err12);
        }
        errors++;
      }
    }
  } else {
    const err13 = { instancePath, schemaPath: "#/oneOf/1/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err13];
    } else {
      vErrors.push(err13);
    }
    errors++;
  }
  var _valid0 = _errs7 === errors;
  if (_valid0 && valid0) {
    valid0 = false;
    passing0 = [passing0, 1];
  } else {
    if (_valid0) {
      valid0 = true;
      passing0 = 1;
      if (props0 !== true) {
        props0 = props0 || {};
        props0.kind = true;
        props0.operationId = true;
        props0.repositoryId = true;
      }
    }
    const _errs15 = errors;
    if (data && typeof data == "object" && !Array.isArray(data)) {
      if (data.kind === void 0) {
        const err14 = { instancePath, schemaPath: "#/oneOf/2/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
        if (vErrors === null) {
          vErrors = [err14];
        } else {
          vErrors.push(err14);
        }
        errors++;
      }
      if (data.repositoryId === void 0) {
        const err15 = { instancePath, schemaPath: "#/oneOf/2/required", keyword: "required", params: { missingProperty: "repositoryId" }, message: "must have required property 'repositoryId'" };
        if (vErrors === null) {
          vErrors = [err15];
        } else {
          vErrors.push(err15);
        }
        errors++;
      }
      if (data.operationId === void 0) {
        const err16 = { instancePath, schemaPath: "#/oneOf/2/required", keyword: "required", params: { missingProperty: "operationId" }, message: "must have required property 'operationId'" };
        if (vErrors === null) {
          vErrors = [err16];
        } else {
          vErrors.push(err16);
        }
        errors++;
      }
      if (data.requirementId === void 0) {
        const err17 = { instancePath, schemaPath: "#/oneOf/2/required", keyword: "required", params: { missingProperty: "requirementId" }, message: "must have required property 'requirementId'" };
        if (vErrors === null) {
          vErrors = [err17];
        } else {
          vErrors.push(err17);
        }
        errors++;
      }
      if (data.capabilityId === void 0) {
        const err18 = { instancePath, schemaPath: "#/oneOf/2/required", keyword: "required", params: { missingProperty: "capabilityId" }, message: "must have required property 'capabilityId'" };
        if (vErrors === null) {
          vErrors = [err18];
        } else {
          vErrors.push(err18);
        }
        errors++;
      }
      if (data.disposition === void 0) {
        const err19 = { instancePath, schemaPath: "#/oneOf/2/required", keyword: "required", params: { missingProperty: "disposition" }, message: "must have required property 'disposition'" };
        if (vErrors === null) {
          vErrors = [err19];
        } else {
          vErrors.push(err19);
        }
        errors++;
      }
      if (data.capabilityId !== void 0) {
        if (typeof data.capabilityId !== "string") {
          const err20 = { instancePath: instancePath + "/capabilityId", schemaPath: "#/oneOf/2/properties/capabilityId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
          if (vErrors === null) {
            vErrors = [err20];
          } else {
            vErrors.push(err20);
          }
          errors++;
        }
      }
      if (data.disposition !== void 0) {
        if (!validate172(data.disposition, { instancePath: instancePath + "/disposition", parentData: data, parentDataProperty: "disposition", rootData, dynamicAnchors })) {
          vErrors = vErrors === null ? validate172.errors : vErrors.concat(validate172.errors);
          errors = vErrors.length;
        }
      }
      if (data.kind !== void 0) {
        let data7 = data.kind;
        if (typeof data7 !== "string") {
          const err21 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/2/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
          if (vErrors === null) {
            vErrors = [err21];
          } else {
            vErrors.push(err21);
          }
          errors++;
        }
        if ("requirement" !== data7) {
          const err22 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/2/properties/kind/const", keyword: "const", params: { allowedValue: "requirement" }, message: "must be equal to constant" };
          if (vErrors === null) {
            vErrors = [err22];
          } else {
            vErrors.push(err22);
          }
          errors++;
        }
      }
      if (data.operationId !== void 0) {
        if (typeof data.operationId !== "string") {
          const err23 = { instancePath: instancePath + "/operationId", schemaPath: "#/oneOf/2/properties/operationId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
          if (vErrors === null) {
            vErrors = [err23];
          } else {
            vErrors.push(err23);
          }
          errors++;
        }
      }
      if (data.repositoryId !== void 0) {
        if (typeof data.repositoryId !== "string") {
          const err24 = { instancePath: instancePath + "/repositoryId", schemaPath: "#/oneOf/2/properties/repositoryId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
          if (vErrors === null) {
            vErrors = [err24];
          } else {
            vErrors.push(err24);
          }
          errors++;
        }
      }
      if (data.requirementId !== void 0) {
        if (typeof data.requirementId !== "string") {
          const err25 = { instancePath: instancePath + "/requirementId", schemaPath: "#/oneOf/2/properties/requirementId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
          if (vErrors === null) {
            vErrors = [err25];
          } else {
            vErrors.push(err25);
          }
          errors++;
        }
      }
    } else {
      const err26 = { instancePath, schemaPath: "#/oneOf/2/type", keyword: "type", params: { type: "object" }, message: "must be object" };
      if (vErrors === null) {
        vErrors = [err26];
      } else {
        vErrors.push(err26);
      }
      errors++;
    }
    var _valid0 = _errs15 === errors;
    if (_valid0 && valid0) {
      valid0 = false;
      passing0 = [passing0, 2];
    } else {
      if (_valid0) {
        valid0 = true;
        passing0 = 2;
        if (props0 !== true) {
          props0 = props0 || {};
          props0.capabilityId = true;
          props0.disposition = true;
          props0.kind = true;
          props0.operationId = true;
          props0.repositoryId = true;
          props0.requirementId = true;
        }
      }
    }
  }
  if (!valid0) {
    const err27 = { instancePath, schemaPath: "#/oneOf", keyword: "oneOf", params: { passingSchemas: passing0 }, message: "must match exactly one schema in oneOf" };
    if (vErrors === null) {
      vErrors = [err27];
    } else {
      vErrors.push(err27);
    }
    errors++;
  } else {
    errors = _errs0;
    if (vErrors !== null) {
      if (_errs0) {
        vErrors.length = _errs0;
      } else {
        vErrors = null;
      }
    }
  }
  validate178.errors = vErrors;
  evaluated0.props = props0;
  return errors === 0;
}
validate178.evaluated = { "dynamicProps": true, "dynamicItems": false };
function validate148(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate148.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  const _errs0 = errors;
  let valid0 = false;
  let passing0 = null;
  const _errs1 = errors;
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.kind === void 0) {
      const err0 = { instancePath, schemaPath: "#/oneOf/0/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
      if (vErrors === null) {
        vErrors = [err0];
      } else {
        vErrors.push(err0);
      }
      errors++;
    }
    if (data.selection === void 0) {
      const err1 = { instancePath, schemaPath: "#/oneOf/0/required", keyword: "required", params: { missingProperty: "selection" }, message: "must have required property 'selection'" };
      if (vErrors === null) {
        vErrors = [err1];
      } else {
        vErrors.push(err1);
      }
      errors++;
    }
    if (data.kind !== void 0) {
      let data0 = data.kind;
      if (typeof data0 !== "string") {
        const err2 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/0/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err2];
        } else {
          vErrors.push(err2);
        }
        errors++;
      }
      if ("declared-selection" !== data0) {
        const err3 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/0/properties/kind/const", keyword: "const", params: { allowedValue: "declared-selection" }, message: "must be equal to constant" };
        if (vErrors === null) {
          vErrors = [err3];
        } else {
          vErrors.push(err3);
        }
        errors++;
      }
    }
    if (data.selection !== void 0) {
      if (!validate149(data.selection, { instancePath: instancePath + "/selection", parentData: data, parentDataProperty: "selection", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate149.errors : vErrors.concat(validate149.errors);
        errors = vErrors.length;
      }
    }
  } else {
    const err4 = { instancePath, schemaPath: "#/oneOf/0/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err4];
    } else {
      vErrors.push(err4);
    }
    errors++;
  }
  var _valid0 = _errs1 === errors;
  if (_valid0) {
    valid0 = true;
    passing0 = 0;
    var props1 = {};
    props1.kind = true;
    props1.selection = true;
  }
  const _errs6 = errors;
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.kind === void 0) {
      const err5 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
      if (vErrors === null) {
        vErrors = [err5];
      } else {
        vErrors.push(err5);
      }
      errors++;
    }
    if (data.contractId === void 0) {
      const err6 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "contractId" }, message: "must have required property 'contractId'" };
      if (vErrors === null) {
        vErrors = [err6];
      } else {
        vErrors.push(err6);
      }
      errors++;
    }
    if (data.executionPathId === void 0) {
      const err7 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "executionPathId" }, message: "must have required property 'executionPathId'" };
      if (vErrors === null) {
        vErrors = [err7];
      } else {
        vErrors.push(err7);
      }
      errors++;
    }
    if (data.reasonSteps === void 0) {
      const err8 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "reasonSteps" }, message: "must have required property 'reasonSteps'" };
      if (vErrors === null) {
        vErrors = [err8];
      } else {
        vErrors.push(err8);
      }
      errors++;
    }
    if (data.contractId !== void 0) {
      if (typeof data.contractId !== "string") {
        const err9 = { instancePath: instancePath + "/contractId", schemaPath: "#/oneOf/1/properties/contractId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err9];
        } else {
          vErrors.push(err9);
        }
        errors++;
      }
    }
    if (data.executionPathId !== void 0) {
      if (typeof data.executionPathId !== "string") {
        const err10 = { instancePath: instancePath + "/executionPathId", schemaPath: "#/oneOf/1/properties/executionPathId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err10];
        } else {
          vErrors.push(err10);
        }
        errors++;
      }
    }
    if (data.kind !== void 0) {
      let data4 = data.kind;
      if (typeof data4 !== "string") {
        const err11 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/1/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err11];
        } else {
          vErrors.push(err11);
        }
        errors++;
      }
      if ("responsibility" !== data4) {
        const err12 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/1/properties/kind/const", keyword: "const", params: { allowedValue: "responsibility" }, message: "must be equal to constant" };
        if (vErrors === null) {
          vErrors = [err12];
        } else {
          vErrors.push(err12);
        }
        errors++;
      }
    }
    if (data.reasonSteps !== void 0) {
      let data5 = data.reasonSteps;
      if (Array.isArray(data5)) {
        const len0 = data5.length;
        for (let i0 = 0; i0 < len0; i0++) {
          if (!validate157(data5[i0], { instancePath: instancePath + "/reasonSteps/" + i0, parentData: data5, parentDataProperty: i0, rootData, dynamicAnchors })) {
            vErrors = vErrors === null ? validate157.errors : vErrors.concat(validate157.errors);
            errors = vErrors.length;
          }
        }
      } else {
        const err13 = { instancePath: instancePath + "/reasonSteps", schemaPath: "#/oneOf/1/properties/reasonSteps/type", keyword: "type", params: { type: "array" }, message: "must be array" };
        if (vErrors === null) {
          vErrors = [err13];
        } else {
          vErrors.push(err13);
        }
        errors++;
      }
    }
  } else {
    const err14 = { instancePath, schemaPath: "#/oneOf/1/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err14];
    } else {
      vErrors.push(err14);
    }
    errors++;
  }
  var _valid0 = _errs6 === errors;
  if (_valid0 && valid0) {
    valid0 = false;
    passing0 = [passing0, 1];
  } else {
    if (_valid0) {
      valid0 = true;
      passing0 = 1;
      if (props1 !== true) {
        props1 = props1 || {};
        props1.contractId = true;
        props1.executionPathId = true;
        props1.kind = true;
        props1.reasonSteps = true;
      }
    }
    const _errs17 = errors;
    if (data && typeof data == "object" && !Array.isArray(data)) {
      if (data.kind === void 0) {
        const err15 = { instancePath, schemaPath: "#/oneOf/2/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
        if (vErrors === null) {
          vErrors = [err15];
        } else {
          vErrors.push(err15);
        }
        errors++;
      }
      if (data.selection === void 0) {
        const err16 = { instancePath, schemaPath: "#/oneOf/2/required", keyword: "required", params: { missingProperty: "selection" }, message: "must have required property 'selection'" };
        if (vErrors === null) {
          vErrors = [err16];
        } else {
          vErrors.push(err16);
        }
        errors++;
      }
      if (data.kind !== void 0) {
        let data7 = data.kind;
        if (typeof data7 !== "string") {
          const err17 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/2/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
          if (vErrors === null) {
            vErrors = [err17];
          } else {
            vErrors.push(err17);
          }
          errors++;
        }
        if ("effective-selection" !== data7) {
          const err18 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/2/properties/kind/const", keyword: "const", params: { allowedValue: "effective-selection" }, message: "must be equal to constant" };
          if (vErrors === null) {
            vErrors = [err18];
          } else {
            vErrors.push(err18);
          }
          errors++;
        }
      }
      if (data.selection !== void 0) {
        if (!validate159(data.selection, { instancePath: instancePath + "/selection", parentData: data, parentDataProperty: "selection", rootData, dynamicAnchors })) {
          vErrors = vErrors === null ? validate159.errors : vErrors.concat(validate159.errors);
          errors = vErrors.length;
        }
      }
    } else {
      const err19 = { instancePath, schemaPath: "#/oneOf/2/type", keyword: "type", params: { type: "object" }, message: "must be object" };
      if (vErrors === null) {
        vErrors = [err19];
      } else {
        vErrors.push(err19);
      }
      errors++;
    }
    var _valid0 = _errs17 === errors;
    if (_valid0 && valid0) {
      valid0 = false;
      passing0 = [passing0, 2];
    } else {
      if (_valid0) {
        valid0 = true;
        passing0 = 2;
        if (props1 !== true) {
          props1 = props1 || {};
          props1.kind = true;
          props1.selection = true;
        }
      }
      const _errs22 = errors;
      if (data && typeof data == "object" && !Array.isArray(data)) {
        if (data.kind === void 0) {
          const err20 = { instancePath, schemaPath: "#/oneOf/3/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
          if (vErrors === null) {
            vErrors = [err20];
          } else {
            vErrors.push(err20);
          }
          errors++;
        }
        if (data.selectionId === void 0) {
          const err21 = { instancePath, schemaPath: "#/oneOf/3/required", keyword: "required", params: { missingProperty: "selectionId" }, message: "must have required property 'selectionId'" };
          if (vErrors === null) {
            vErrors = [err21];
          } else {
            vErrors.push(err21);
          }
          errors++;
        }
        if (data.reasonSteps === void 0) {
          const err22 = { instancePath, schemaPath: "#/oneOf/3/required", keyword: "required", params: { missingProperty: "reasonSteps" }, message: "must have required property 'reasonSteps'" };
          if (vErrors === null) {
            vErrors = [err22];
          } else {
            vErrors.push(err22);
          }
          errors++;
        }
        if (data.kind !== void 0) {
          let data9 = data.kind;
          if (typeof data9 !== "string") {
            const err23 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/3/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
            if (vErrors === null) {
              vErrors = [err23];
            } else {
              vErrors.push(err23);
            }
            errors++;
          }
          if ("execution-path" !== data9) {
            const err24 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/3/properties/kind/const", keyword: "const", params: { allowedValue: "execution-path" }, message: "must be equal to constant" };
            if (vErrors === null) {
              vErrors = [err24];
            } else {
              vErrors.push(err24);
            }
            errors++;
          }
        }
        if (data.reasonSteps !== void 0) {
          let data10 = data.reasonSteps;
          if (Array.isArray(data10)) {
            const len1 = data10.length;
            for (let i1 = 0; i1 < len1; i1++) {
              if (!validate157(data10[i1], { instancePath: instancePath + "/reasonSteps/" + i1, parentData: data10, parentDataProperty: i1, rootData, dynamicAnchors })) {
                vErrors = vErrors === null ? validate157.errors : vErrors.concat(validate157.errors);
                errors = vErrors.length;
              }
            }
          } else {
            const err25 = { instancePath: instancePath + "/reasonSteps", schemaPath: "#/oneOf/3/properties/reasonSteps/type", keyword: "type", params: { type: "array" }, message: "must be array" };
            if (vErrors === null) {
              vErrors = [err25];
            } else {
              vErrors.push(err25);
            }
            errors++;
          }
        }
        if (data.selectionId !== void 0) {
          if (typeof data.selectionId !== "string") {
            const err26 = { instancePath: instancePath + "/selectionId", schemaPath: "#/oneOf/3/properties/selectionId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
            if (vErrors === null) {
              vErrors = [err26];
            } else {
              vErrors.push(err26);
            }
            errors++;
          }
        }
      } else {
        const err27 = { instancePath, schemaPath: "#/oneOf/3/type", keyword: "type", params: { type: "object" }, message: "must be object" };
        if (vErrors === null) {
          vErrors = [err27];
        } else {
          vErrors.push(err27);
        }
        errors++;
      }
      var _valid0 = _errs22 === errors;
      if (_valid0 && valid0) {
        valid0 = false;
        passing0 = [passing0, 3];
      } else {
        if (_valid0) {
          valid0 = true;
          passing0 = 3;
          if (props1 !== true) {
            props1 = props1 || {};
            props1.kind = true;
            props1.reasonSteps = true;
            props1.selectionId = true;
          }
        }
        const _errs31 = errors;
        if (data && typeof data == "object" && !Array.isArray(data)) {
          if (data.kind === void 0) {
            const err28 = { instancePath, schemaPath: "#/oneOf/4/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
            if (vErrors === null) {
              vErrors = [err28];
            } else {
              vErrors.push(err28);
            }
            errors++;
          }
          if (data.exposure === void 0) {
            const err29 = { instancePath, schemaPath: "#/oneOf/4/required", keyword: "required", params: { missingProperty: "exposure" }, message: "must have required property 'exposure'" };
            if (vErrors === null) {
              vErrors = [err29];
            } else {
              vErrors.push(err29);
            }
            errors++;
          }
          if (data.nativeRealization === void 0) {
            const err30 = { instancePath, schemaPath: "#/oneOf/4/required", keyword: "required", params: { missingProperty: "nativeRealization" }, message: "must have required property 'nativeRealization'" };
            if (vErrors === null) {
              vErrors = [err30];
            } else {
              vErrors.push(err30);
            }
            errors++;
          }
          if (data.exposure !== void 0) {
            if (!validate163(data.exposure, { instancePath: instancePath + "/exposure", parentData: data, parentDataProperty: "exposure", rootData, dynamicAnchors })) {
              vErrors = vErrors === null ? validate163.errors : vErrors.concat(validate163.errors);
              errors = vErrors.length;
            }
          }
          if (data.kind !== void 0) {
            let data14 = data.kind;
            if (typeof data14 !== "string") {
              const err31 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/4/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
              if (vErrors === null) {
                vErrors = [err31];
              } else {
                vErrors.push(err31);
              }
              errors++;
            }
            if ("operation-classification" !== data14) {
              const err32 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/4/properties/kind/const", keyword: "const", params: { allowedValue: "operation-classification" }, message: "must be equal to constant" };
              if (vErrors === null) {
                vErrors = [err32];
              } else {
                vErrors.push(err32);
              }
              errors++;
            }
          }
          if (data.nativeRealization !== void 0) {
            if (!validate165(data.nativeRealization, { instancePath: instancePath + "/nativeRealization", parentData: data, parentDataProperty: "nativeRealization", rootData, dynamicAnchors })) {
              vErrors = vErrors === null ? validate165.errors : vErrors.concat(validate165.errors);
              errors = vErrors.length;
            }
          }
        } else {
          const err33 = { instancePath, schemaPath: "#/oneOf/4/type", keyword: "type", params: { type: "object" }, message: "must be object" };
          if (vErrors === null) {
            vErrors = [err33];
          } else {
            vErrors.push(err33);
          }
          errors++;
        }
        var _valid0 = _errs31 === errors;
        if (_valid0 && valid0) {
          valid0 = false;
          passing0 = [passing0, 4];
        } else {
          if (_valid0) {
            valid0 = true;
            passing0 = 4;
            if (props1 !== true) {
              props1 = props1 || {};
              props1.exposure = true;
              props1.kind = true;
              props1.nativeRealization = true;
            }
          }
          const _errs37 = errors;
          if (data && typeof data == "object" && !Array.isArray(data)) {
            if (data.kind === void 0) {
              const err34 = { instancePath, schemaPath: "#/oneOf/5/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
              if (vErrors === null) {
                vErrors = [err34];
              } else {
                vErrors.push(err34);
              }
              errors++;
            }
            if (data.requirementDisposition === void 0) {
              const err35 = { instancePath, schemaPath: "#/oneOf/5/required", keyword: "required", params: { missingProperty: "requirementDisposition" }, message: "must have required property 'requirementDisposition'" };
              if (vErrors === null) {
                vErrors = [err35];
              } else {
                vErrors.push(err35);
              }
              errors++;
            }
            if (data.providerDisposition === void 0) {
              const err36 = { instancePath, schemaPath: "#/oneOf/5/required", keyword: "required", params: { missingProperty: "providerDisposition" }, message: "must have required property 'providerDisposition'" };
              if (vErrors === null) {
                vErrors = [err36];
              } else {
                vErrors.push(err36);
              }
              errors++;
            }
            if (data.nativeRealization === void 0) {
              const err37 = { instancePath, schemaPath: "#/oneOf/5/required", keyword: "required", params: { missingProperty: "nativeRealization" }, message: "must have required property 'nativeRealization'" };
              if (vErrors === null) {
                vErrors = [err37];
              } else {
                vErrors.push(err37);
              }
              errors++;
            }
            if (data.kind !== void 0) {
              let data16 = data.kind;
              if (typeof data16 !== "string") {
                const err38 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/5/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                if (vErrors === null) {
                  vErrors = [err38];
                } else {
                  vErrors.push(err38);
                }
                errors++;
              }
              if ("capability-classification" !== data16) {
                const err39 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/5/properties/kind/const", keyword: "const", params: { allowedValue: "capability-classification" }, message: "must be equal to constant" };
                if (vErrors === null) {
                  vErrors = [err39];
                } else {
                  vErrors.push(err39);
                }
                errors++;
              }
            }
            if (data.nativeRealization !== void 0) {
              if (!validate165(data.nativeRealization, { instancePath: instancePath + "/nativeRealization", parentData: data, parentDataProperty: "nativeRealization", rootData, dynamicAnchors })) {
                vErrors = vErrors === null ? validate165.errors : vErrors.concat(validate165.errors);
                errors = vErrors.length;
              }
            }
            if (data.providerDisposition !== void 0) {
              if (!validate168(data.providerDisposition, { instancePath: instancePath + "/providerDisposition", parentData: data, parentDataProperty: "providerDisposition", rootData, dynamicAnchors })) {
                vErrors = vErrors === null ? validate168.errors : vErrors.concat(validate168.errors);
                errors = vErrors.length;
              }
            }
            if (data.requirementDisposition !== void 0) {
              if (!validate172(data.requirementDisposition, { instancePath: instancePath + "/requirementDisposition", parentData: data, parentDataProperty: "requirementDisposition", rootData, dynamicAnchors })) {
                vErrors = vErrors === null ? validate172.errors : vErrors.concat(validate172.errors);
                errors = vErrors.length;
              }
            }
          } else {
            const err40 = { instancePath, schemaPath: "#/oneOf/5/type", keyword: "type", params: { type: "object" }, message: "must be object" };
            if (vErrors === null) {
              vErrors = [err40];
            } else {
              vErrors.push(err40);
            }
            errors++;
          }
          var _valid0 = _errs37 === errors;
          if (_valid0 && valid0) {
            valid0 = false;
            passing0 = [passing0, 5];
          } else {
            if (_valid0) {
              valid0 = true;
              passing0 = 5;
              if (props1 !== true) {
                props1 = props1 || {};
                props1.kind = true;
                props1.nativeRealization = true;
                props1.providerDisposition = true;
                props1.requirementDisposition = true;
              }
            }
            const _errs44 = errors;
            if (data && typeof data == "object" && !Array.isArray(data)) {
              if (data.kind === void 0) {
                const err41 = { instancePath, schemaPath: "#/oneOf/6/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
                if (vErrors === null) {
                  vErrors = [err41];
                } else {
                  vErrors.push(err41);
                }
                errors++;
              }
              if (data.finding === void 0) {
                const err42 = { instancePath, schemaPath: "#/oneOf/6/required", keyword: "required", params: { missingProperty: "finding" }, message: "must have required property 'finding'" };
                if (vErrors === null) {
                  vErrors = [err42];
                } else {
                  vErrors.push(err42);
                }
                errors++;
              }
              if (data.finding !== void 0) {
                if (!validate178(data.finding, { instancePath: instancePath + "/finding", parentData: data, parentDataProperty: "finding", rootData, dynamicAnchors })) {
                  vErrors = vErrors === null ? validate178.errors : vErrors.concat(validate178.errors);
                  errors = vErrors.length;
                }
              }
              if (data.kind !== void 0) {
                let data21 = data.kind;
                if (typeof data21 !== "string") {
                  const err43 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/6/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                  if (vErrors === null) {
                    vErrors = [err43];
                  } else {
                    vErrors.push(err43);
                  }
                  errors++;
                }
                if ("finding-classification" !== data21) {
                  const err44 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/6/properties/kind/const", keyword: "const", params: { allowedValue: "finding-classification" }, message: "must be equal to constant" };
                  if (vErrors === null) {
                    vErrors = [err44];
                  } else {
                    vErrors.push(err44);
                  }
                  errors++;
                }
              }
            } else {
              const err45 = { instancePath, schemaPath: "#/oneOf/6/type", keyword: "type", params: { type: "object" }, message: "must be object" };
              if (vErrors === null) {
                vErrors = [err45];
              } else {
                vErrors.push(err45);
              }
              errors++;
            }
            var _valid0 = _errs44 === errors;
            if (_valid0 && valid0) {
              valid0 = false;
              passing0 = [passing0, 6];
            } else {
              if (_valid0) {
                valid0 = true;
                passing0 = 6;
                if (props1 !== true) {
                  props1 = props1 || {};
                  props1.finding = true;
                  props1.kind = true;
                }
              }
              const _errs49 = errors;
              if (data && typeof data == "object" && !Array.isArray(data)) {
                if (data.kind === void 0) {
                  const err46 = { instancePath, schemaPath: "#/oneOf/7/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
                  if (vErrors === null) {
                    vErrors = [err46];
                  } else {
                    vErrors.push(err46);
                  }
                  errors++;
                }
                if (data.realization === void 0) {
                  const err47 = { instancePath, schemaPath: "#/oneOf/7/required", keyword: "required", params: { missingProperty: "realization" }, message: "must have required property 'realization'" };
                  if (vErrors === null) {
                    vErrors = [err47];
                  } else {
                    vErrors.push(err47);
                  }
                  errors++;
                }
                if (data.kind !== void 0) {
                  let data22 = data.kind;
                  if (typeof data22 !== "string") {
                    const err48 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/7/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                    if (vErrors === null) {
                      vErrors = [err48];
                    } else {
                      vErrors.push(err48);
                    }
                    errors++;
                  }
                  if ("machine-realization" !== data22) {
                    const err49 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/7/properties/kind/const", keyword: "const", params: { allowedValue: "machine-realization" }, message: "must be equal to constant" };
                    if (vErrors === null) {
                      vErrors = [err49];
                    } else {
                      vErrors.push(err49);
                    }
                    errors++;
                  }
                }
                if (data.realization !== void 0) {
                  if (!validate165(data.realization, { instancePath: instancePath + "/realization", parentData: data, parentDataProperty: "realization", rootData, dynamicAnchors })) {
                    vErrors = vErrors === null ? validate165.errors : vErrors.concat(validate165.errors);
                    errors = vErrors.length;
                  }
                }
              } else {
                const err50 = { instancePath, schemaPath: "#/oneOf/7/type", keyword: "type", params: { type: "object" }, message: "must be object" };
                if (vErrors === null) {
                  vErrors = [err50];
                } else {
                  vErrors.push(err50);
                }
                errors++;
              }
              var _valid0 = _errs49 === errors;
              if (_valid0 && valid0) {
                valid0 = false;
                passing0 = [passing0, 7];
              } else {
                if (_valid0) {
                  valid0 = true;
                  passing0 = 7;
                  if (props1 !== true) {
                    props1 = props1 || {};
                    props1.kind = true;
                    props1.realization = true;
                  }
                }
                const _errs54 = errors;
                if (data && typeof data == "object" && !Array.isArray(data)) {
                  if (data.kind === void 0) {
                    const err51 = { instancePath, schemaPath: "#/oneOf/8/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
                    if (vErrors === null) {
                      vErrors = [err51];
                    } else {
                      vErrors.push(err51);
                    }
                    errors++;
                  }
                  if (data.kind !== void 0) {
                    let data24 = data.kind;
                    if (typeof data24 !== "string") {
                      const err52 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/8/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                      if (vErrors === null) {
                        vErrors = [err52];
                      } else {
                        vErrors.push(err52);
                      }
                      errors++;
                    }
                    if ("configuration" !== data24) {
                      const err53 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/8/properties/kind/const", keyword: "const", params: { allowedValue: "configuration" }, message: "must be equal to constant" };
                      if (vErrors === null) {
                        vErrors = [err53];
                      } else {
                        vErrors.push(err53);
                      }
                      errors++;
                    }
                  }
                } else {
                  const err54 = { instancePath, schemaPath: "#/oneOf/8/type", keyword: "type", params: { type: "object" }, message: "must be object" };
                  if (vErrors === null) {
                    vErrors = [err54];
                  } else {
                    vErrors.push(err54);
                  }
                  errors++;
                }
                var _valid0 = _errs54 === errors;
                if (_valid0 && valid0) {
                  valid0 = false;
                  passing0 = [passing0, 8];
                } else {
                  if (_valid0) {
                    valid0 = true;
                    passing0 = 8;
                    if (props1 !== true) {
                      props1 = props1 || {};
                      props1.kind = true;
                    }
                  }
                }
              }
            }
          }
        }
      }
    }
  }
  if (!valid0) {
    const err55 = { instancePath, schemaPath: "#/oneOf", keyword: "oneOf", params: { passingSchemas: passing0 }, message: "must match exactly one schema in oneOf" };
    if (vErrors === null) {
      vErrors = [err55];
    } else {
      vErrors.push(err55);
    }
    errors++;
  } else {
    errors = _errs0;
    if (vErrors !== null) {
      if (_errs0) {
        vErrors.length = _errs0;
      } else {
        vErrors = null;
      }
    }
  }
  validate148.errors = vErrors;
  evaluated0.props = props1;
  return errors === 0;
}
validate148.evaluated = { "dynamicProps": true, "dynamicItems": false };
var pattern3 = new RegExp("^[0-9a-f]{64}$", "u");
function validate183(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate183.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  if (typeof data === "string") {
    if (func2(data) > 64) {
      const err0 = { instancePath, schemaPath: "#/maxLength", keyword: "maxLength", params: { limit: 64 }, message: "must NOT have more than 64 characters" };
      if (vErrors === null) {
        vErrors = [err0];
      } else {
        vErrors.push(err0);
      }
      errors++;
    }
    if (func2(data) < 64) {
      const err1 = { instancePath, schemaPath: "#/minLength", keyword: "minLength", params: { limit: 64 }, message: "must NOT have fewer than 64 characters" };
      if (vErrors === null) {
        vErrors = [err1];
      } else {
        vErrors.push(err1);
      }
      errors++;
    }
    if (!pattern3.test(data)) {
      const err2 = { instancePath, schemaPath: "#/pattern", keyword: "pattern", params: { pattern: "^[0-9a-f]{64}$" }, message: 'must match pattern "^[0-9a-f]{64}$"' };
      if (vErrors === null) {
        vErrors = [err2];
      } else {
        vErrors.push(err2);
      }
      errors++;
    }
  } else {
    const err3 = { instancePath, schemaPath: "#/type", keyword: "type", params: { type: "string" }, message: "must be string" };
    if (vErrors === null) {
      vErrors = [err3];
    } else {
      vErrors.push(err3);
    }
    errors++;
  }
  validate183.errors = vErrors;
  return errors === 0;
}
validate183.evaluated = { "dynamicProps": false, "dynamicItems": false };
function validate147(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate147.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.handle === void 0) {
      const err0 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "handle" }, message: "must have required property 'handle'" };
      if (vErrors === null) {
        vErrors = [err0];
      } else {
        vErrors.push(err0);
      }
      errors++;
    }
    if (data.evidence === void 0) {
      const err1 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "evidence" }, message: "must have required property 'evidence'" };
      if (vErrors === null) {
        vErrors = [err1];
      } else {
        vErrors.push(err1);
      }
      errors++;
    }
    for (const key0 in data) {
      if (!(key0 === "evidence" || key0 === "handle")) {
        const err2 = { instancePath, schemaPath: "#/additionalProperties", keyword: "additionalProperties", params: { additionalProperty: key0 }, message: "must NOT have additional properties" };
        if (vErrors === null) {
          vErrors = [err2];
        } else {
          vErrors.push(err2);
        }
        errors++;
      }
    }
    if (data.evidence !== void 0) {
      if (!validate148(data.evidence, { instancePath: instancePath + "/evidence", parentData: data, parentDataProperty: "evidence", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate148.errors : vErrors.concat(validate148.errors);
        errors = vErrors.length;
      }
    }
    if (data.handle !== void 0) {
      if (!validate183(data.handle, { instancePath: instancePath + "/handle", parentData: data, parentDataProperty: "handle", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate183.errors : vErrors.concat(validate183.errors);
        errors = vErrors.length;
      }
    }
  } else {
    const err3 = { instancePath, schemaPath: "#/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err3];
    } else {
      vErrors.push(err3);
    }
    errors++;
  }
  validate147.errors = vErrors;
  return errors === 0;
}
validate147.evaluated = { "props": true, "dynamicProps": false, "dynamicItems": false };
function validate268(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate268.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.editor === void 0) {
      const err0 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "editor" }, message: "must have required property 'editor'" };
      if (vErrors === null) {
        vErrors = [err0];
      } else {
        vErrors.push(err0);
      }
      errors++;
    }
    if (data.context === void 0) {
      const err1 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "context" }, message: "must have required property 'context'" };
      if (vErrors === null) {
        vErrors = [err1];
      } else {
        vErrors.push(err1);
      }
      errors++;
    }
    if (data.coordinates === void 0) {
      const err2 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "coordinates" }, message: "must have required property 'coordinates'" };
      if (vErrors === null) {
        vErrors = [err2];
      } else {
        vErrors.push(err2);
      }
      errors++;
    }
    if (data.explanation === void 0) {
      const err3 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "explanation" }, message: "must have required property 'explanation'" };
      if (vErrors === null) {
        vErrors = [err3];
      } else {
        vErrors.push(err3);
      }
      errors++;
    }
    for (const key0 in data) {
      if (!(key0 === "context" || key0 === "coordinates" || key0 === "editor" || key0 === "explanation")) {
        const err4 = { instancePath, schemaPath: "#/additionalProperties", keyword: "additionalProperties", params: { additionalProperty: key0 }, message: "must NOT have additional properties" };
        if (vErrors === null) {
          vErrors = [err4];
        } else {
          vErrors.push(err4);
        }
        errors++;
      }
    }
    if (data.context !== void 0) {
      if (!validate79(data.context, { instancePath: instancePath + "/context", parentData: data, parentDataProperty: "context", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate79.errors : vErrors.concat(validate79.errors);
        errors = vErrors.length;
      }
    }
    if (data.coordinates !== void 0) {
      if (!validate144(data.coordinates, { instancePath: instancePath + "/coordinates", parentData: data, parentDataProperty: "coordinates", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate144.errors : vErrors.concat(validate144.errors);
        errors = vErrors.length;
      }
    }
    if (data.editor !== void 0) {
      if (!validate56(data.editor, { instancePath: instancePath + "/editor", parentData: data, parentDataProperty: "editor", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate56.errors : vErrors.concat(validate56.errors);
        errors = vErrors.length;
      }
    }
    if (data.explanation !== void 0) {
      if (!validate147(data.explanation, { instancePath: instancePath + "/explanation", parentData: data, parentDataProperty: "explanation", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate147.errors : vErrors.concat(validate147.errors);
        errors = vErrors.length;
      }
    }
  } else {
    const err5 = { instancePath, schemaPath: "#/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err5];
    } else {
      vErrors.push(err5);
    }
    errors++;
  }
  validate268.errors = vErrors;
  return errors === 0;
}
validate268.evaluated = { "props": true, "dynamicProps": false, "dynamicItems": false };
var validateEffectiveProfileResponse = validate273;
function validate190(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate190.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  if (typeof data === "string") {
    if (func2(data) > 64) {
      const err0 = { instancePath, schemaPath: "#/maxLength", keyword: "maxLength", params: { limit: 64 }, message: "must NOT have more than 64 characters" };
      if (vErrors === null) {
        vErrors = [err0];
      } else {
        vErrors.push(err0);
      }
      errors++;
    }
    if (func2(data) < 64) {
      const err1 = { instancePath, schemaPath: "#/minLength", keyword: "minLength", params: { limit: 64 }, message: "must NOT have fewer than 64 characters" };
      if (vErrors === null) {
        vErrors = [err1];
      } else {
        vErrors.push(err1);
      }
      errors++;
    }
    if (!pattern3.test(data)) {
      const err2 = { instancePath, schemaPath: "#/pattern", keyword: "pattern", params: { pattern: "^[0-9a-f]{64}$" }, message: 'must match pattern "^[0-9a-f]{64}$"' };
      if (vErrors === null) {
        vErrors = [err2];
      } else {
        vErrors.push(err2);
      }
      errors++;
    }
  } else {
    const err3 = { instancePath, schemaPath: "#/type", keyword: "type", params: { type: "string" }, message: "must be string" };
    if (vErrors === null) {
      vErrors = [err3];
    } else {
      vErrors.push(err3);
    }
    errors++;
  }
  validate190.errors = vErrors;
  return errors === 0;
}
validate190.evaluated = { "dynamicProps": false, "dynamicItems": false };
var schema73 = { "additionalProperties": false, "description": "One deterministic page from a retained effective-profile projection.", "properties": { "entries": { "description": "Complete entries in the requested bounded window.", "items": { "$ref": "#/$defs/EffectiveProfileEntry" }, "type": "array" }, "nextOffset": { "description": "Next zero-based offset, or `None` for the terminal page.", "format": "uint32", "minimum": 0, "type": ["integer", "null"] }, "offset": { "description": "Exact zero-based offset requested for this page.\n\nThis remains the requested coordinate even when it lies beyond the\nterminal entry. Language transports use it to reject cross-page reply\nsubstitution without interpreting profile content.", "format": "uint32", "minimum": 0, "type": "integer" }, "section": { "$ref": "#/$defs/EffectiveProfileSection", "description": "Section represented by this page." } }, "required": ["section", "offset", "entries", "nextOffset"], "type": "object" };
function validate196(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate196.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  const _errs0 = errors;
  let valid0 = false;
  let passing0 = null;
  const _errs1 = errors;
  if (typeof data !== "string") {
    const err0 = { instancePath, schemaPath: "#/oneOf/0/type", keyword: "type", params: { type: "string" }, message: "must be string" };
    if (vErrors === null) {
      vErrors = [err0];
    } else {
      vErrors.push(err0);
    }
    errors++;
  }
  if ("compute" !== data) {
    const err1 = { instancePath, schemaPath: "#/oneOf/0/const", keyword: "const", params: { allowedValue: "compute" }, message: "must be equal to constant" };
    if (vErrors === null) {
      vErrors = [err1];
    } else {
      vErrors.push(err1);
    }
    errors++;
  }
  var _valid0 = _errs1 === errors;
  if (_valid0) {
    valid0 = true;
    passing0 = 0;
  }
  const _errs3 = errors;
  if (typeof data !== "string") {
    const err2 = { instancePath, schemaPath: "#/oneOf/1/type", keyword: "type", params: { type: "string" }, message: "must be string" };
    if (vErrors === null) {
      vErrors = [err2];
    } else {
      vErrors.push(err2);
    }
    errors++;
  }
  if ("read" !== data) {
    const err3 = { instancePath, schemaPath: "#/oneOf/1/const", keyword: "const", params: { allowedValue: "read" }, message: "must be equal to constant" };
    if (vErrors === null) {
      vErrors = [err3];
    } else {
      vErrors.push(err3);
    }
    errors++;
  }
  var _valid0 = _errs3 === errors;
  if (_valid0 && valid0) {
    valid0 = false;
    passing0 = [passing0, 1];
  } else {
    if (_valid0) {
      valid0 = true;
      passing0 = 1;
    }
    const _errs5 = errors;
    if (typeof data !== "string") {
      const err4 = { instancePath, schemaPath: "#/oneOf/2/type", keyword: "type", params: { type: "string" }, message: "must be string" };
      if (vErrors === null) {
        vErrors = [err4];
      } else {
        vErrors.push(err4);
      }
      errors++;
    }
    if ("execute" !== data) {
      const err5 = { instancePath, schemaPath: "#/oneOf/2/const", keyword: "const", params: { allowedValue: "execute" }, message: "must be equal to constant" };
      if (vErrors === null) {
        vErrors = [err5];
      } else {
        vErrors.push(err5);
      }
      errors++;
    }
    var _valid0 = _errs5 === errors;
    if (_valid0 && valid0) {
      valid0 = false;
      passing0 = [passing0, 2];
    } else {
      if (_valid0) {
        valid0 = true;
        passing0 = 2;
      }
      const _errs7 = errors;
      if (typeof data !== "string") {
        const err6 = { instancePath, schemaPath: "#/oneOf/3/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err6];
        } else {
          vErrors.push(err6);
        }
        errors++;
      }
      if ("write" !== data) {
        const err7 = { instancePath, schemaPath: "#/oneOf/3/const", keyword: "const", params: { allowedValue: "write" }, message: "must be equal to constant" };
        if (vErrors === null) {
          vErrors = [err7];
        } else {
          vErrors.push(err7);
        }
        errors++;
      }
      var _valid0 = _errs7 === errors;
      if (_valid0 && valid0) {
        valid0 = false;
        passing0 = [passing0, 3];
      } else {
        if (_valid0) {
          valid0 = true;
          passing0 = 3;
        }
        const _errs9 = errors;
        if (typeof data !== "string") {
          const err8 = { instancePath, schemaPath: "#/oneOf/4/type", keyword: "type", params: { type: "string" }, message: "must be string" };
          if (vErrors === null) {
            vErrors = [err8];
          } else {
            vErrors.push(err8);
          }
          errors++;
        }
        if ("network" !== data) {
          const err9 = { instancePath, schemaPath: "#/oneOf/4/const", keyword: "const", params: { allowedValue: "network" }, message: "must be equal to constant" };
          if (vErrors === null) {
            vErrors = [err9];
          } else {
            vErrors.push(err9);
          }
          errors++;
        }
        var _valid0 = _errs9 === errors;
        if (_valid0 && valid0) {
          valid0 = false;
          passing0 = [passing0, 4];
        } else {
          if (_valid0) {
            valid0 = true;
            passing0 = 4;
          }
          const _errs11 = errors;
          if (typeof data !== "string") {
            const err10 = { instancePath, schemaPath: "#/oneOf/5/type", keyword: "type", params: { type: "string" }, message: "must be string" };
            if (vErrors === null) {
              vErrors = [err10];
            } else {
              vErrors.push(err10);
            }
            errors++;
          }
          if ("publish" !== data) {
            const err11 = { instancePath, schemaPath: "#/oneOf/5/const", keyword: "const", params: { allowedValue: "publish" }, message: "must be equal to constant" };
            if (vErrors === null) {
              vErrors = [err11];
            } else {
              vErrors.push(err11);
            }
            errors++;
          }
          var _valid0 = _errs11 === errors;
          if (_valid0 && valid0) {
            valid0 = false;
            passing0 = [passing0, 5];
          } else {
            if (_valid0) {
              valid0 = true;
              passing0 = 5;
            }
          }
        }
      }
    }
  }
  if (!valid0) {
    const err12 = { instancePath, schemaPath: "#/oneOf", keyword: "oneOf", params: { passingSchemas: passing0 }, message: "must match exactly one schema in oneOf" };
    if (vErrors === null) {
      vErrors = [err12];
    } else {
      vErrors.push(err12);
    }
    errors++;
  } else {
    errors = _errs0;
    if (vErrors !== null) {
      if (_errs0) {
        vErrors.length = _errs0;
      } else {
        vErrors = null;
      }
    }
  }
  validate196.errors = vErrors;
  return errors === 0;
}
validate196.evaluated = { "dynamicProps": false, "dynamicItems": false };
function validate198(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate198.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  const _errs0 = errors;
  let valid0 = false;
  let passing0 = null;
  const _errs1 = errors;
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.kind === void 0) {
      const err0 = { instancePath, schemaPath: "#/oneOf/0/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
      if (vErrors === null) {
        vErrors = [err0];
      } else {
        vErrors.push(err0);
      }
      errors++;
    }
    if (data.interfaceId === void 0) {
      const err1 = { instancePath, schemaPath: "#/oneOf/0/required", keyword: "required", params: { missingProperty: "interfaceId" }, message: "must have required property 'interfaceId'" };
      if (vErrors === null) {
        vErrors = [err1];
      } else {
        vErrors.push(err1);
      }
      errors++;
    }
    if (data.interfaceId !== void 0) {
      if (typeof data.interfaceId !== "string") {
        const err2 = { instancePath: instancePath + "/interfaceId", schemaPath: "#/oneOf/0/properties/interfaceId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err2];
        } else {
          vErrors.push(err2);
        }
        errors++;
      }
    }
    if (data.kind !== void 0) {
      let data1 = data.kind;
      if (typeof data1 !== "string") {
        const err3 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/0/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err3];
        } else {
          vErrors.push(err3);
        }
        errors++;
      }
      if ("interface" !== data1) {
        const err4 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/0/properties/kind/const", keyword: "const", params: { allowedValue: "interface" }, message: "must be equal to constant" };
        if (vErrors === null) {
          vErrors = [err4];
        } else {
          vErrors.push(err4);
        }
        errors++;
      }
    }
  } else {
    const err5 = { instancePath, schemaPath: "#/oneOf/0/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err5];
    } else {
      vErrors.push(err5);
    }
    errors++;
  }
  var _valid0 = _errs1 === errors;
  if (_valid0) {
    valid0 = true;
    passing0 = 0;
    var props0 = {};
    props0.interfaceId = true;
    props0.kind = true;
  }
  const _errs7 = errors;
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.kind === void 0) {
      const err6 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
      if (vErrors === null) {
        vErrors = [err6];
      } else {
        vErrors.push(err6);
      }
      errors++;
    }
    if (data.applicationId === void 0) {
      const err7 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "applicationId" }, message: "must have required property 'applicationId'" };
      if (vErrors === null) {
        vErrors = [err7];
      } else {
        vErrors.push(err7);
      }
      errors++;
    }
    if (data.applicationId !== void 0) {
      if (typeof data.applicationId !== "string") {
        const err8 = { instancePath: instancePath + "/applicationId", schemaPath: "#/oneOf/1/properties/applicationId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err8];
        } else {
          vErrors.push(err8);
        }
        errors++;
      }
    }
    if (data.kind !== void 0) {
      let data3 = data.kind;
      if (typeof data3 !== "string") {
        const err9 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/1/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err9];
        } else {
          vErrors.push(err9);
        }
        errors++;
      }
      if ("application" !== data3) {
        const err10 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/1/properties/kind/const", keyword: "const", params: { allowedValue: "application" }, message: "must be equal to constant" };
        if (vErrors === null) {
          vErrors = [err10];
        } else {
          vErrors.push(err10);
        }
        errors++;
      }
    }
  } else {
    const err11 = { instancePath, schemaPath: "#/oneOf/1/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err11];
    } else {
      vErrors.push(err11);
    }
    errors++;
  }
  var _valid0 = _errs7 === errors;
  if (_valid0 && valid0) {
    valid0 = false;
    passing0 = [passing0, 1];
  } else {
    if (_valid0) {
      valid0 = true;
      passing0 = 1;
      if (props0 !== true) {
        props0 = props0 || {};
        props0.applicationId = true;
        props0.kind = true;
      }
    }
    const _errs13 = errors;
    if (data && typeof data == "object" && !Array.isArray(data)) {
      if (data.kind === void 0) {
        const err12 = { instancePath, schemaPath: "#/oneOf/2/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
        if (vErrors === null) {
          vErrors = [err12];
        } else {
          vErrors.push(err12);
        }
        errors++;
      }
      if (data.engineId === void 0) {
        const err13 = { instancePath, schemaPath: "#/oneOf/2/required", keyword: "required", params: { missingProperty: "engineId" }, message: "must have required property 'engineId'" };
        if (vErrors === null) {
          vErrors = [err13];
        } else {
          vErrors.push(err13);
        }
        errors++;
      }
      if (data.engineId !== void 0) {
        if (typeof data.engineId !== "string") {
          const err14 = { instancePath: instancePath + "/engineId", schemaPath: "#/oneOf/2/properties/engineId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
          if (vErrors === null) {
            vErrors = [err14];
          } else {
            vErrors.push(err14);
          }
          errors++;
        }
      }
      if (data.kind !== void 0) {
        let data5 = data.kind;
        if (typeof data5 !== "string") {
          const err15 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/2/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
          if (vErrors === null) {
            vErrors = [err15];
          } else {
            vErrors.push(err15);
          }
          errors++;
        }
        if ("engine" !== data5) {
          const err16 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/2/properties/kind/const", keyword: "const", params: { allowedValue: "engine" }, message: "must be equal to constant" };
          if (vErrors === null) {
            vErrors = [err16];
          } else {
            vErrors.push(err16);
          }
          errors++;
        }
      }
    } else {
      const err17 = { instancePath, schemaPath: "#/oneOf/2/type", keyword: "type", params: { type: "object" }, message: "must be object" };
      if (vErrors === null) {
        vErrors = [err17];
      } else {
        vErrors.push(err17);
      }
      errors++;
    }
    var _valid0 = _errs13 === errors;
    if (_valid0 && valid0) {
      valid0 = false;
      passing0 = [passing0, 2];
    } else {
      if (_valid0) {
        valid0 = true;
        passing0 = 2;
        if (props0 !== true) {
          props0 = props0 || {};
          props0.engineId = true;
          props0.kind = true;
        }
      }
      const _errs19 = errors;
      if (data && typeof data == "object" && !Array.isArray(data)) {
        if (data.kind === void 0) {
          const err18 = { instancePath, schemaPath: "#/oneOf/3/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
          if (vErrors === null) {
            vErrors = [err18];
          } else {
            vErrors.push(err18);
          }
          errors++;
        }
        if (data.capabilityId === void 0) {
          const err19 = { instancePath, schemaPath: "#/oneOf/3/required", keyword: "required", params: { missingProperty: "capabilityId" }, message: "must have required property 'capabilityId'" };
          if (vErrors === null) {
            vErrors = [err19];
          } else {
            vErrors.push(err19);
          }
          errors++;
        }
        if (data.capabilityId !== void 0) {
          if (typeof data.capabilityId !== "string") {
            const err20 = { instancePath: instancePath + "/capabilityId", schemaPath: "#/oneOf/3/properties/capabilityId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
            if (vErrors === null) {
              vErrors = [err20];
            } else {
              vErrors.push(err20);
            }
            errors++;
          }
        }
        if (data.kind !== void 0) {
          let data7 = data.kind;
          if (typeof data7 !== "string") {
            const err21 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/3/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
            if (vErrors === null) {
              vErrors = [err21];
            } else {
              vErrors.push(err21);
            }
            errors++;
          }
          if ("capability" !== data7) {
            const err22 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/3/properties/kind/const", keyword: "const", params: { allowedValue: "capability" }, message: "must be equal to constant" };
            if (vErrors === null) {
              vErrors = [err22];
            } else {
              vErrors.push(err22);
            }
            errors++;
          }
        }
      } else {
        const err23 = { instancePath, schemaPath: "#/oneOf/3/type", keyword: "type", params: { type: "object" }, message: "must be object" };
        if (vErrors === null) {
          vErrors = [err23];
        } else {
          vErrors.push(err23);
        }
        errors++;
      }
      var _valid0 = _errs19 === errors;
      if (_valid0 && valid0) {
        valid0 = false;
        passing0 = [passing0, 3];
      } else {
        if (_valid0) {
          valid0 = true;
          passing0 = 3;
          if (props0 !== true) {
            props0 = props0 || {};
            props0.capabilityId = true;
            props0.kind = true;
          }
        }
        const _errs25 = errors;
        if (data && typeof data == "object" && !Array.isArray(data)) {
          if (data.kind === void 0) {
            const err24 = { instancePath, schemaPath: "#/oneOf/4/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
            if (vErrors === null) {
              vErrors = [err24];
            } else {
              vErrors.push(err24);
            }
            errors++;
          }
          if (data.terminalLeafId === void 0) {
            const err25 = { instancePath, schemaPath: "#/oneOf/4/required", keyword: "required", params: { missingProperty: "terminalLeafId" }, message: "must have required property 'terminalLeafId'" };
            if (vErrors === null) {
              vErrors = [err25];
            } else {
              vErrors.push(err25);
            }
            errors++;
          }
          if (data.kind !== void 0) {
            let data8 = data.kind;
            if (typeof data8 !== "string") {
              const err26 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/4/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
              if (vErrors === null) {
                vErrors = [err26];
              } else {
                vErrors.push(err26);
              }
              errors++;
            }
            if ("terminal-leaf" !== data8) {
              const err27 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/4/properties/kind/const", keyword: "const", params: { allowedValue: "terminal-leaf" }, message: "must be equal to constant" };
              if (vErrors === null) {
                vErrors = [err27];
              } else {
                vErrors.push(err27);
              }
              errors++;
            }
          }
          if (data.terminalLeafId !== void 0) {
            if (typeof data.terminalLeafId !== "string") {
              const err28 = { instancePath: instancePath + "/terminalLeafId", schemaPath: "#/oneOf/4/properties/terminalLeafId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
              if (vErrors === null) {
                vErrors = [err28];
              } else {
                vErrors.push(err28);
              }
              errors++;
            }
          }
        } else {
          const err29 = { instancePath, schemaPath: "#/oneOf/4/type", keyword: "type", params: { type: "object" }, message: "must be object" };
          if (vErrors === null) {
            vErrors = [err29];
          } else {
            vErrors.push(err29);
          }
          errors++;
        }
        var _valid0 = _errs25 === errors;
        if (_valid0 && valid0) {
          valid0 = false;
          passing0 = [passing0, 4];
        } else {
          if (_valid0) {
            valid0 = true;
            passing0 = 4;
            if (props0 !== true) {
              props0 = props0 || {};
              props0.kind = true;
              props0.terminalLeafId = true;
            }
          }
          const _errs31 = errors;
          if (data && typeof data == "object" && !Array.isArray(data)) {
            if (data.kind === void 0) {
              const err30 = { instancePath, schemaPath: "#/oneOf/5/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
              if (vErrors === null) {
                vErrors = [err30];
              } else {
                vErrors.push(err30);
              }
              errors++;
            }
            if (data.pluginId === void 0) {
              const err31 = { instancePath, schemaPath: "#/oneOf/5/required", keyword: "required", params: { missingProperty: "pluginId" }, message: "must have required property 'pluginId'" };
              if (vErrors === null) {
                vErrors = [err31];
              } else {
                vErrors.push(err31);
              }
              errors++;
            }
            if (data.kind !== void 0) {
              let data10 = data.kind;
              if (typeof data10 !== "string") {
                const err32 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/5/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                if (vErrors === null) {
                  vErrors = [err32];
                } else {
                  vErrors.push(err32);
                }
                errors++;
              }
              if ("plugin" !== data10) {
                const err33 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/5/properties/kind/const", keyword: "const", params: { allowedValue: "plugin" }, message: "must be equal to constant" };
                if (vErrors === null) {
                  vErrors = [err33];
                } else {
                  vErrors.push(err33);
                }
                errors++;
              }
            }
            if (data.pluginId !== void 0) {
              if (typeof data.pluginId !== "string") {
                const err34 = { instancePath: instancePath + "/pluginId", schemaPath: "#/oneOf/5/properties/pluginId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                if (vErrors === null) {
                  vErrors = [err34];
                } else {
                  vErrors.push(err34);
                }
                errors++;
              }
            }
          } else {
            const err35 = { instancePath, schemaPath: "#/oneOf/5/type", keyword: "type", params: { type: "object" }, message: "must be object" };
            if (vErrors === null) {
              vErrors = [err35];
            } else {
              vErrors.push(err35);
            }
            errors++;
          }
          var _valid0 = _errs31 === errors;
          if (_valid0 && valid0) {
            valid0 = false;
            passing0 = [passing0, 5];
          } else {
            if (_valid0) {
              valid0 = true;
              passing0 = 5;
              if (props0 !== true) {
                props0 = props0 || {};
                props0.kind = true;
                props0.pluginId = true;
              }
            }
            const _errs37 = errors;
            if (data && typeof data == "object" && !Array.isArray(data)) {
              if (data.kind === void 0) {
                const err36 = { instancePath, schemaPath: "#/oneOf/6/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
                if (vErrors === null) {
                  vErrors = [err36];
                } else {
                  vErrors.push(err36);
                }
                errors++;
              }
              if (data.skillId === void 0) {
                const err37 = { instancePath, schemaPath: "#/oneOf/6/required", keyword: "required", params: { missingProperty: "skillId" }, message: "must have required property 'skillId'" };
                if (vErrors === null) {
                  vErrors = [err37];
                } else {
                  vErrors.push(err37);
                }
                errors++;
              }
              if (data.kind !== void 0) {
                let data12 = data.kind;
                if (typeof data12 !== "string") {
                  const err38 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/6/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                  if (vErrors === null) {
                    vErrors = [err38];
                  } else {
                    vErrors.push(err38);
                  }
                  errors++;
                }
                if ("skill" !== data12) {
                  const err39 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/6/properties/kind/const", keyword: "const", params: { allowedValue: "skill" }, message: "must be equal to constant" };
                  if (vErrors === null) {
                    vErrors = [err39];
                  } else {
                    vErrors.push(err39);
                  }
                  errors++;
                }
              }
              if (data.skillId !== void 0) {
                if (typeof data.skillId !== "string") {
                  const err40 = { instancePath: instancePath + "/skillId", schemaPath: "#/oneOf/6/properties/skillId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                  if (vErrors === null) {
                    vErrors = [err40];
                  } else {
                    vErrors.push(err40);
                  }
                  errors++;
                }
              }
            } else {
              const err41 = { instancePath, schemaPath: "#/oneOf/6/type", keyword: "type", params: { type: "object" }, message: "must be object" };
              if (vErrors === null) {
                vErrors = [err41];
              } else {
                vErrors.push(err41);
              }
              errors++;
            }
            var _valid0 = _errs37 === errors;
            if (_valid0 && valid0) {
              valid0 = false;
              passing0 = [passing0, 6];
            } else {
              if (_valid0) {
                valid0 = true;
                passing0 = 6;
                if (props0 !== true) {
                  props0 = props0 || {};
                  props0.kind = true;
                  props0.skillId = true;
                }
              }
            }
          }
        }
      }
    }
  }
  if (!valid0) {
    const err42 = { instancePath, schemaPath: "#/oneOf", keyword: "oneOf", params: { passingSchemas: passing0 }, message: "must match exactly one schema in oneOf" };
    if (vErrors === null) {
      vErrors = [err42];
    } else {
      vErrors.push(err42);
    }
    errors++;
  } else {
    errors = _errs0;
    if (vErrors !== null) {
      if (_errs0) {
        vErrors.length = _errs0;
      } else {
        vErrors = null;
      }
    }
  }
  validate198.errors = vErrors;
  evaluated0.props = props0;
  return errors === 0;
}
validate198.evaluated = { "dynamicProps": true, "dynamicItems": false };
function validate201(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate201.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  const _errs0 = errors;
  let valid0 = false;
  let passing0 = null;
  const _errs1 = errors;
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.kind === void 0) {
      const err0 = { instancePath, schemaPath: "#/oneOf/0/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
      if (vErrors === null) {
        vErrors = [err0];
      } else {
        vErrors.push(err0);
      }
      errors++;
    }
    if (data.kind !== void 0) {
      let data0 = data.kind;
      if (typeof data0 !== "string") {
        const err1 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/0/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err1];
        } else {
          vErrors.push(err1);
        }
        errors++;
      }
      if ("communication" !== data0) {
        const err2 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/0/properties/kind/const", keyword: "const", params: { allowedValue: "communication" }, message: "must be equal to constant" };
        if (vErrors === null) {
          vErrors = [err2];
        } else {
          vErrors.push(err2);
        }
        errors++;
      }
    }
  } else {
    const err3 = { instancePath, schemaPath: "#/oneOf/0/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err3];
    } else {
      vErrors.push(err3);
    }
    errors++;
  }
  var _valid0 = _errs1 === errors;
  if (_valid0) {
    valid0 = true;
    passing0 = 0;
    var props0 = {};
    props0.kind = true;
  }
  const _errs5 = errors;
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.kind === void 0) {
      const err4 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
      if (vErrors === null) {
        vErrors = [err4];
      } else {
        vErrors.push(err4);
      }
      errors++;
    }
    if (data.kind !== void 0) {
      let data1 = data.kind;
      if (typeof data1 !== "string") {
        const err5 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/1/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err5];
        } else {
          vErrors.push(err5);
        }
        errors++;
      }
      if ("organizational-work" !== data1) {
        const err6 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/1/properties/kind/const", keyword: "const", params: { allowedValue: "organizational-work" }, message: "must be equal to constant" };
        if (vErrors === null) {
          vErrors = [err6];
        } else {
          vErrors.push(err6);
        }
        errors++;
      }
    }
  } else {
    const err7 = { instancePath, schemaPath: "#/oneOf/1/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err7];
    } else {
      vErrors.push(err7);
    }
    errors++;
  }
  var _valid0 = _errs5 === errors;
  if (_valid0 && valid0) {
    valid0 = false;
    passing0 = [passing0, 1];
  } else {
    if (_valid0) {
      valid0 = true;
      passing0 = 1;
      if (props0 !== true) {
        props0 = props0 || {};
        props0.kind = true;
      }
    }
    const _errs9 = errors;
    if (data && typeof data == "object" && !Array.isArray(data)) {
      if (data.kind === void 0) {
        const err8 = { instancePath, schemaPath: "#/oneOf/2/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
        if (vErrors === null) {
          vErrors = [err8];
        } else {
          vErrors.push(err8);
        }
        errors++;
      }
      if (data.kind !== void 0) {
        let data2 = data.kind;
        if (typeof data2 !== "string") {
          const err9 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/2/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
          if (vErrors === null) {
            vErrors = [err9];
          } else {
            vErrors.push(err9);
          }
          errors++;
        }
        if ("crm-business-state" !== data2) {
          const err10 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/2/properties/kind/const", keyword: "const", params: { allowedValue: "crm-business-state" }, message: "must be equal to constant" };
          if (vErrors === null) {
            vErrors = [err10];
          } else {
            vErrors.push(err10);
          }
          errors++;
        }
      }
    } else {
      const err11 = { instancePath, schemaPath: "#/oneOf/2/type", keyword: "type", params: { type: "object" }, message: "must be object" };
      if (vErrors === null) {
        vErrors = [err11];
      } else {
        vErrors.push(err11);
      }
      errors++;
    }
    var _valid0 = _errs9 === errors;
    if (_valid0 && valid0) {
      valid0 = false;
      passing0 = [passing0, 2];
    } else {
      if (_valid0) {
        valid0 = true;
        passing0 = 2;
        if (props0 !== true) {
          props0 = props0 || {};
          props0.kind = true;
        }
      }
      const _errs13 = errors;
      if (data && typeof data == "object" && !Array.isArray(data)) {
        if (data.kind === void 0) {
          const err12 = { instancePath, schemaPath: "#/oneOf/3/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
          if (vErrors === null) {
            vErrors = [err12];
          } else {
            vErrors.push(err12);
          }
          errors++;
        }
        if (data.kind !== void 0) {
          let data3 = data.kind;
          if (typeof data3 !== "string") {
            const err13 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/3/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
            if (vErrors === null) {
              vErrors = [err13];
            } else {
              vErrors.push(err13);
            }
            errors++;
          }
          if ("engineering-execution" !== data3) {
            const err14 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/3/properties/kind/const", keyword: "const", params: { allowedValue: "engineering-execution" }, message: "must be equal to constant" };
            if (vErrors === null) {
              vErrors = [err14];
            } else {
              vErrors.push(err14);
            }
            errors++;
          }
        }
      } else {
        const err15 = { instancePath, schemaPath: "#/oneOf/3/type", keyword: "type", params: { type: "object" }, message: "must be object" };
        if (vErrors === null) {
          vErrors = [err15];
        } else {
          vErrors.push(err15);
        }
        errors++;
      }
      var _valid0 = _errs13 === errors;
      if (_valid0 && valid0) {
        valid0 = false;
        passing0 = [passing0, 3];
      } else {
        if (_valid0) {
          valid0 = true;
          passing0 = 3;
          if (props0 !== true) {
            props0 = props0 || {};
            props0.kind = true;
          }
        }
        const _errs17 = errors;
        if (data && typeof data == "object" && !Array.isArray(data)) {
          if (data.kind === void 0) {
            const err16 = { instancePath, schemaPath: "#/oneOf/4/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
            if (vErrors === null) {
              vErrors = [err16];
            } else {
              vErrors.push(err16);
            }
            errors++;
          }
          if (data.kind !== void 0) {
            let data4 = data.kind;
            if (typeof data4 !== "string") {
              const err17 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/4/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
              if (vErrors === null) {
                vErrors = [err17];
              } else {
                vErrors.push(err17);
              }
              errors++;
            }
            if ("context-memory" !== data4) {
              const err18 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/4/properties/kind/const", keyword: "const", params: { allowedValue: "context-memory" }, message: "must be equal to constant" };
              if (vErrors === null) {
                vErrors = [err18];
              } else {
                vErrors.push(err18);
              }
              errors++;
            }
          }
        } else {
          const err19 = { instancePath, schemaPath: "#/oneOf/4/type", keyword: "type", params: { type: "object" }, message: "must be object" };
          if (vErrors === null) {
            vErrors = [err19];
          } else {
            vErrors.push(err19);
          }
          errors++;
        }
        var _valid0 = _errs17 === errors;
        if (_valid0 && valid0) {
          valid0 = false;
          passing0 = [passing0, 4];
        } else {
          if (_valid0) {
            valid0 = true;
            passing0 = 4;
            if (props0 !== true) {
              props0 = props0 || {};
              props0.kind = true;
            }
          }
          const _errs21 = errors;
          if (data && typeof data == "object" && !Array.isArray(data)) {
            if (data.kind === void 0) {
              const err20 = { instancePath, schemaPath: "#/oneOf/5/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
              if (vErrors === null) {
                vErrors = [err20];
              } else {
                vErrors.push(err20);
              }
              errors++;
            }
            if (data.kind !== void 0) {
              let data5 = data.kind;
              if (typeof data5 !== "string") {
                const err21 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/5/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                if (vErrors === null) {
                  vErrors = [err21];
                } else {
                  vErrors.push(err21);
                }
                errors++;
              }
              if ("terminal-harness-execution" !== data5) {
                const err22 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/5/properties/kind/const", keyword: "const", params: { allowedValue: "terminal-harness-execution" }, message: "must be equal to constant" };
                if (vErrors === null) {
                  vErrors = [err22];
                } else {
                  vErrors.push(err22);
                }
                errors++;
              }
            }
          } else {
            const err23 = { instancePath, schemaPath: "#/oneOf/5/type", keyword: "type", params: { type: "object" }, message: "must be object" };
            if (vErrors === null) {
              vErrors = [err23];
            } else {
              vErrors.push(err23);
            }
            errors++;
          }
          var _valid0 = _errs21 === errors;
          if (_valid0 && valid0) {
            valid0 = false;
            passing0 = [passing0, 5];
          } else {
            if (_valid0) {
              valid0 = true;
              passing0 = 5;
              if (props0 !== true) {
                props0 = props0 || {};
                props0.kind = true;
              }
            }
            const _errs25 = errors;
            if (data && typeof data == "object" && !Array.isArray(data)) {
              if (data.kind === void 0) {
                const err24 = { instancePath, schemaPath: "#/oneOf/6/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
                if (vErrors === null) {
                  vErrors = [err24];
                } else {
                  vErrors.push(err24);
                }
                errors++;
              }
              if (data.kind !== void 0) {
                let data6 = data.kind;
                if (typeof data6 !== "string") {
                  const err25 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/6/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                  if (vErrors === null) {
                    vErrors = [err25];
                  } else {
                    vErrors.push(err25);
                  }
                  errors++;
                }
                if ("native-event-publication" !== data6) {
                  const err26 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/6/properties/kind/const", keyword: "const", params: { allowedValue: "native-event-publication" }, message: "must be equal to constant" };
                  if (vErrors === null) {
                    vErrors = [err26];
                  } else {
                    vErrors.push(err26);
                  }
                  errors++;
                }
              }
            } else {
              const err27 = { instancePath, schemaPath: "#/oneOf/6/type", keyword: "type", params: { type: "object" }, message: "must be object" };
              if (vErrors === null) {
                vErrors = [err27];
              } else {
                vErrors.push(err27);
              }
              errors++;
            }
            var _valid0 = _errs25 === errors;
            if (_valid0 && valid0) {
              valid0 = false;
              passing0 = [passing0, 6];
            } else {
              if (_valid0) {
                valid0 = true;
                passing0 = 6;
                if (props0 !== true) {
                  props0 = props0 || {};
                  props0.kind = true;
                }
              }
              const _errs29 = errors;
              if (data && typeof data == "object" && !Array.isArray(data)) {
                if (data.kind === void 0) {
                  const err28 = { instancePath, schemaPath: "#/oneOf/7/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
                  if (vErrors === null) {
                    vErrors = [err28];
                  } else {
                    vErrors.push(err28);
                  }
                  errors++;
                }
                if (data.kind !== void 0) {
                  let data7 = data.kind;
                  if (typeof data7 !== "string") {
                    const err29 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/7/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                    if (vErrors === null) {
                      vErrors = [err29];
                    } else {
                      vErrors.push(err29);
                    }
                    errors++;
                  }
                  if ("native-effect-read-back" !== data7) {
                    const err30 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/7/properties/kind/const", keyword: "const", params: { allowedValue: "native-effect-read-back" }, message: "must be equal to constant" };
                    if (vErrors === null) {
                      vErrors = [err30];
                    } else {
                      vErrors.push(err30);
                    }
                    errors++;
                  }
                }
              } else {
                const err31 = { instancePath, schemaPath: "#/oneOf/7/type", keyword: "type", params: { type: "object" }, message: "must be object" };
                if (vErrors === null) {
                  vErrors = [err31];
                } else {
                  vErrors.push(err31);
                }
                errors++;
              }
              var _valid0 = _errs29 === errors;
              if (_valid0 && valid0) {
                valid0 = false;
                passing0 = [passing0, 7];
              } else {
                if (_valid0) {
                  valid0 = true;
                  passing0 = 7;
                  if (props0 !== true) {
                    props0 = props0 || {};
                    props0.kind = true;
                  }
                }
                const _errs33 = errors;
                if (data && typeof data == "object" && !Array.isArray(data)) {
                  if (data.kind === void 0) {
                    const err32 = { instancePath, schemaPath: "#/oneOf/8/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
                    if (vErrors === null) {
                      vErrors = [err32];
                    } else {
                      vErrors.push(err32);
                    }
                    errors++;
                  }
                  if (data.namespaceId === void 0) {
                    const err33 = { instancePath, schemaPath: "#/oneOf/8/required", keyword: "required", params: { missingProperty: "namespaceId" }, message: "must have required property 'namespaceId'" };
                    if (vErrors === null) {
                      vErrors = [err33];
                    } else {
                      vErrors.push(err33);
                    }
                    errors++;
                  }
                  if (data.contractId === void 0) {
                    const err34 = { instancePath, schemaPath: "#/oneOf/8/required", keyword: "required", params: { missingProperty: "contractId" }, message: "must have required property 'contractId'" };
                    if (vErrors === null) {
                      vErrors = [err34];
                    } else {
                      vErrors.push(err34);
                    }
                    errors++;
                  }
                  if (data.schemaId === void 0) {
                    const err35 = { instancePath, schemaPath: "#/oneOf/8/required", keyword: "required", params: { missingProperty: "schemaId" }, message: "must have required property 'schemaId'" };
                    if (vErrors === null) {
                      vErrors = [err35];
                    } else {
                      vErrors.push(err35);
                    }
                    errors++;
                  }
                  if (data.schemaSha256 === void 0) {
                    const err36 = { instancePath, schemaPath: "#/oneOf/8/required", keyword: "required", params: { missingProperty: "schemaSha256" }, message: "must have required property 'schemaSha256'" };
                    if (vErrors === null) {
                      vErrors = [err36];
                    } else {
                      vErrors.push(err36);
                    }
                    errors++;
                  }
                  if (data.contractId !== void 0) {
                    if (typeof data.contractId !== "string") {
                      const err37 = { instancePath: instancePath + "/contractId", schemaPath: "#/oneOf/8/properties/contractId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                      if (vErrors === null) {
                        vErrors = [err37];
                      } else {
                        vErrors.push(err37);
                      }
                      errors++;
                    }
                  }
                  if (data.kind !== void 0) {
                    let data9 = data.kind;
                    if (typeof data9 !== "string") {
                      const err38 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/8/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                      if (vErrors === null) {
                        vErrors = [err38];
                      } else {
                        vErrors.push(err38);
                      }
                      errors++;
                    }
                    if ("extension" !== data9) {
                      const err39 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/8/properties/kind/const", keyword: "const", params: { allowedValue: "extension" }, message: "must be equal to constant" };
                      if (vErrors === null) {
                        vErrors = [err39];
                      } else {
                        vErrors.push(err39);
                      }
                      errors++;
                    }
                  }
                  if (data.namespaceId !== void 0) {
                    if (typeof data.namespaceId !== "string") {
                      const err40 = { instancePath: instancePath + "/namespaceId", schemaPath: "#/oneOf/8/properties/namespaceId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                      if (vErrors === null) {
                        vErrors = [err40];
                      } else {
                        vErrors.push(err40);
                      }
                      errors++;
                    }
                  }
                  if (data.schemaId !== void 0) {
                    if (typeof data.schemaId !== "string") {
                      const err41 = { instancePath: instancePath + "/schemaId", schemaPath: "#/oneOf/8/properties/schemaId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                      if (vErrors === null) {
                        vErrors = [err41];
                      } else {
                        vErrors.push(err41);
                      }
                      errors++;
                    }
                  }
                  if (data.schemaSha256 !== void 0) {
                    if (typeof data.schemaSha256 !== "string") {
                      const err42 = { instancePath: instancePath + "/schemaSha256", schemaPath: "#/oneOf/8/properties/schemaSha256/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                      if (vErrors === null) {
                        vErrors = [err42];
                      } else {
                        vErrors.push(err42);
                      }
                      errors++;
                    }
                  }
                } else {
                  const err43 = { instancePath, schemaPath: "#/oneOf/8/type", keyword: "type", params: { type: "object" }, message: "must be object" };
                  if (vErrors === null) {
                    vErrors = [err43];
                  } else {
                    vErrors.push(err43);
                  }
                  errors++;
                }
                var _valid0 = _errs33 === errors;
                if (_valid0 && valid0) {
                  valid0 = false;
                  passing0 = [passing0, 8];
                } else {
                  if (_valid0) {
                    valid0 = true;
                    passing0 = 8;
                    if (props0 !== true) {
                      props0 = props0 || {};
                      props0.contractId = true;
                      props0.kind = true;
                      props0.namespaceId = true;
                      props0.schemaId = true;
                      props0.schemaSha256 = true;
                    }
                  }
                }
              }
            }
          }
        }
      }
    }
  }
  if (!valid0) {
    const err44 = { instancePath, schemaPath: "#/oneOf", keyword: "oneOf", params: { passingSchemas: passing0 }, message: "must match exactly one schema in oneOf" };
    if (vErrors === null) {
      vErrors = [err44];
    } else {
      vErrors.push(err44);
    }
    errors++;
  } else {
    errors = _errs0;
    if (vErrors !== null) {
      if (_errs0) {
        vErrors.length = _errs0;
      } else {
        vErrors = null;
      }
    }
  }
  validate201.errors = vErrors;
  evaluated0.props = props0;
  return errors === 0;
}
validate201.evaluated = { "dynamicProps": true, "dynamicItems": false };
function validate203(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate203.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  const _errs0 = errors;
  let valid0 = false;
  let passing0 = null;
  const _errs1 = errors;
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.kind === void 0) {
      const err0 = { instancePath, schemaPath: "#/oneOf/0/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
      if (vErrors === null) {
        vErrors = [err0];
      } else {
        vErrors.push(err0);
      }
      errors++;
    }
    if (data.sourceId === void 0) {
      const err1 = { instancePath, schemaPath: "#/oneOf/0/required", keyword: "required", params: { missingProperty: "sourceId" }, message: "must have required property 'sourceId'" };
      if (vErrors === null) {
        vErrors = [err1];
      } else {
        vErrors.push(err1);
      }
      errors++;
    }
    if (data.kind !== void 0) {
      let data0 = data.kind;
      if (typeof data0 !== "string") {
        const err2 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/0/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err2];
        } else {
          vErrors.push(err2);
        }
        errors++;
      }
      if ("source" !== data0) {
        const err3 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/0/properties/kind/const", keyword: "const", params: { allowedValue: "source" }, message: "must be equal to constant" };
        if (vErrors === null) {
          vErrors = [err3];
        } else {
          vErrors.push(err3);
        }
        errors++;
      }
    }
    if (data.sourceId !== void 0) {
      if (typeof data.sourceId !== "string") {
        const err4 = { instancePath: instancePath + "/sourceId", schemaPath: "#/oneOf/0/properties/sourceId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err4];
        } else {
          vErrors.push(err4);
        }
        errors++;
      }
    }
  } else {
    const err5 = { instancePath, schemaPath: "#/oneOf/0/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err5];
    } else {
      vErrors.push(err5);
    }
    errors++;
  }
  var _valid0 = _errs1 === errors;
  if (_valid0) {
    valid0 = true;
    passing0 = 0;
    var props0 = {};
    props0.kind = true;
    props0.sourceId = true;
  }
  const _errs7 = errors;
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.kind === void 0) {
      const err6 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
      if (vErrors === null) {
        vErrors = [err6];
      } else {
        vErrors.push(err6);
      }
      errors++;
    }
    if (data.componentId === void 0) {
      const err7 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "componentId" }, message: "must have required property 'componentId'" };
      if (vErrors === null) {
        vErrors = [err7];
      } else {
        vErrors.push(err7);
      }
      errors++;
    }
    if (data.componentId !== void 0) {
      if (typeof data.componentId !== "string") {
        const err8 = { instancePath: instancePath + "/componentId", schemaPath: "#/oneOf/1/properties/componentId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err8];
        } else {
          vErrors.push(err8);
        }
        errors++;
      }
    }
    if (data.kind !== void 0) {
      let data3 = data.kind;
      if (typeof data3 !== "string") {
        const err9 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/1/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err9];
        } else {
          vErrors.push(err9);
        }
        errors++;
      }
      if ("component" !== data3) {
        const err10 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/1/properties/kind/const", keyword: "const", params: { allowedValue: "component" }, message: "must be equal to constant" };
        if (vErrors === null) {
          vErrors = [err10];
        } else {
          vErrors.push(err10);
        }
        errors++;
      }
    }
  } else {
    const err11 = { instancePath, schemaPath: "#/oneOf/1/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err11];
    } else {
      vErrors.push(err11);
    }
    errors++;
  }
  var _valid0 = _errs7 === errors;
  if (_valid0 && valid0) {
    valid0 = false;
    passing0 = [passing0, 1];
  } else {
    if (_valid0) {
      valid0 = true;
      passing0 = 1;
      if (props0 !== true) {
        props0 = props0 || {};
        props0.componentId = true;
        props0.kind = true;
      }
    }
    const _errs13 = errors;
    if (data && typeof data == "object" && !Array.isArray(data)) {
      if (data.kind === void 0) {
        const err12 = { instancePath, schemaPath: "#/oneOf/2/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
        if (vErrors === null) {
          vErrors = [err12];
        } else {
          vErrors.push(err12);
        }
        errors++;
      }
      if (data.moduleId === void 0) {
        const err13 = { instancePath, schemaPath: "#/oneOf/2/required", keyword: "required", params: { missingProperty: "moduleId" }, message: "must have required property 'moduleId'" };
        if (vErrors === null) {
          vErrors = [err13];
        } else {
          vErrors.push(err13);
        }
        errors++;
      }
      if (data.kind !== void 0) {
        let data4 = data.kind;
        if (typeof data4 !== "string") {
          const err14 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/2/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
          if (vErrors === null) {
            vErrors = [err14];
          } else {
            vErrors.push(err14);
          }
          errors++;
        }
        if ("module" !== data4) {
          const err15 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/2/properties/kind/const", keyword: "const", params: { allowedValue: "module" }, message: "must be equal to constant" };
          if (vErrors === null) {
            vErrors = [err15];
          } else {
            vErrors.push(err15);
          }
          errors++;
        }
      }
      if (data.moduleId !== void 0) {
        if (typeof data.moduleId !== "string") {
          const err16 = { instancePath: instancePath + "/moduleId", schemaPath: "#/oneOf/2/properties/moduleId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
          if (vErrors === null) {
            vErrors = [err16];
          } else {
            vErrors.push(err16);
          }
          errors++;
        }
      }
    } else {
      const err17 = { instancePath, schemaPath: "#/oneOf/2/type", keyword: "type", params: { type: "object" }, message: "must be object" };
      if (vErrors === null) {
        vErrors = [err17];
      } else {
        vErrors.push(err17);
      }
      errors++;
    }
    var _valid0 = _errs13 === errors;
    if (_valid0 && valid0) {
      valid0 = false;
      passing0 = [passing0, 2];
    } else {
      if (_valid0) {
        valid0 = true;
        passing0 = 2;
        if (props0 !== true) {
          props0 = props0 || {};
          props0.kind = true;
          props0.moduleId = true;
        }
      }
      const _errs19 = errors;
      if (data && typeof data == "object" && !Array.isArray(data)) {
        if (data.kind === void 0) {
          const err18 = { instancePath, schemaPath: "#/oneOf/3/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
          if (vErrors === null) {
            vErrors = [err18];
          } else {
            vErrors.push(err18);
          }
          errors++;
        }
        if (data.ownerModuleId === void 0) {
          const err19 = { instancePath, schemaPath: "#/oneOf/3/required", keyword: "required", params: { missingProperty: "ownerModuleId" }, message: "must have required property 'ownerModuleId'" };
          if (vErrors === null) {
            vErrors = [err19];
          } else {
            vErrors.push(err19);
          }
          errors++;
        }
        if (data.namespaceId === void 0) {
          const err20 = { instancePath, schemaPath: "#/oneOf/3/required", keyword: "required", params: { missingProperty: "namespaceId" }, message: "must have required property 'namespaceId'" };
          if (vErrors === null) {
            vErrors = [err20];
          } else {
            vErrors.push(err20);
          }
          errors++;
        }
        if (data.declarationId === void 0) {
          const err21 = { instancePath, schemaPath: "#/oneOf/3/required", keyword: "required", params: { missingProperty: "declarationId" }, message: "must have required property 'declarationId'" };
          if (vErrors === null) {
            vErrors = [err21];
          } else {
            vErrors.push(err21);
          }
          errors++;
        }
        if (data.declarationId !== void 0) {
          if (typeof data.declarationId !== "string") {
            const err22 = { instancePath: instancePath + "/declarationId", schemaPath: "#/oneOf/3/properties/declarationId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
            if (vErrors === null) {
              vErrors = [err22];
            } else {
              vErrors.push(err22);
            }
            errors++;
          }
        }
        if (data.kind !== void 0) {
          let data7 = data.kind;
          if (typeof data7 !== "string") {
            const err23 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/3/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
            if (vErrors === null) {
              vErrors = [err23];
            } else {
              vErrors.push(err23);
            }
            errors++;
          }
          if ("declaration" !== data7) {
            const err24 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/3/properties/kind/const", keyword: "const", params: { allowedValue: "declaration" }, message: "must be equal to constant" };
            if (vErrors === null) {
              vErrors = [err24];
            } else {
              vErrors.push(err24);
            }
            errors++;
          }
        }
        if (data.namespaceId !== void 0) {
          if (typeof data.namespaceId !== "string") {
            const err25 = { instancePath: instancePath + "/namespaceId", schemaPath: "#/oneOf/3/properties/namespaceId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
            if (vErrors === null) {
              vErrors = [err25];
            } else {
              vErrors.push(err25);
            }
            errors++;
          }
        }
        if (data.ownerModuleId !== void 0) {
          if (typeof data.ownerModuleId !== "string") {
            const err26 = { instancePath: instancePath + "/ownerModuleId", schemaPath: "#/oneOf/3/properties/ownerModuleId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
            if (vErrors === null) {
              vErrors = [err26];
            } else {
              vErrors.push(err26);
            }
            errors++;
          }
        }
      } else {
        const err27 = { instancePath, schemaPath: "#/oneOf/3/type", keyword: "type", params: { type: "object" }, message: "must be object" };
        if (vErrors === null) {
          vErrors = [err27];
        } else {
          vErrors.push(err27);
        }
        errors++;
      }
      var _valid0 = _errs19 === errors;
      if (_valid0 && valid0) {
        valid0 = false;
        passing0 = [passing0, 3];
      } else {
        if (_valid0) {
          valid0 = true;
          passing0 = 3;
          if (props0 !== true) {
            props0 = props0 || {};
            props0.declarationId = true;
            props0.kind = true;
            props0.namespaceId = true;
            props0.ownerModuleId = true;
          }
        }
        const _errs29 = errors;
        if (data && typeof data == "object" && !Array.isArray(data)) {
          if (data.kind === void 0) {
            const err28 = { instancePath, schemaPath: "#/oneOf/4/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
            if (vErrors === null) {
              vErrors = [err28];
            } else {
              vErrors.push(err28);
            }
            errors++;
          }
          if (data.importerModuleId === void 0) {
            const err29 = { instancePath, schemaPath: "#/oneOf/4/required", keyword: "required", params: { missingProperty: "importerModuleId" }, message: "must have required property 'importerModuleId'" };
            if (vErrors === null) {
              vErrors = [err29];
            } else {
              vErrors.push(err29);
            }
            errors++;
          }
          if (data.upstreamModuleId === void 0) {
            const err30 = { instancePath, schemaPath: "#/oneOf/4/required", keyword: "required", params: { missingProperty: "upstreamModuleId" }, message: "must have required property 'upstreamModuleId'" };
            if (vErrors === null) {
              vErrors = [err30];
            } else {
              vErrors.push(err30);
            }
            errors++;
          }
          if (data.upstreamReleaseId === void 0) {
            const err31 = { instancePath, schemaPath: "#/oneOf/4/required", keyword: "required", params: { missingProperty: "upstreamReleaseId" }, message: "must have required property 'upstreamReleaseId'" };
            if (vErrors === null) {
              vErrors = [err31];
            } else {
              vErrors.push(err31);
            }
            errors++;
          }
          if (data.exportId === void 0) {
            const err32 = { instancePath, schemaPath: "#/oneOf/4/required", keyword: "required", params: { missingProperty: "exportId" }, message: "must have required property 'exportId'" };
            if (vErrors === null) {
              vErrors = [err32];
            } else {
              vErrors.push(err32);
            }
            errors++;
          }
          if (data.sourceNamespaceId === void 0) {
            const err33 = { instancePath, schemaPath: "#/oneOf/4/required", keyword: "required", params: { missingProperty: "sourceNamespaceId" }, message: "must have required property 'sourceNamespaceId'" };
            if (vErrors === null) {
              vErrors = [err33];
            } else {
              vErrors.push(err33);
            }
            errors++;
          }
          if (data.localNamespaceId === void 0) {
            const err34 = { instancePath, schemaPath: "#/oneOf/4/required", keyword: "required", params: { missingProperty: "localNamespaceId" }, message: "must have required property 'localNamespaceId'" };
            if (vErrors === null) {
              vErrors = [err34];
            } else {
              vErrors.push(err34);
            }
            errors++;
          }
          if (data.exportId !== void 0) {
            if (typeof data.exportId !== "string") {
              const err35 = { instancePath: instancePath + "/exportId", schemaPath: "#/oneOf/4/properties/exportId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
              if (vErrors === null) {
                vErrors = [err35];
              } else {
                vErrors.push(err35);
              }
              errors++;
            }
          }
          if (data.importerModuleId !== void 0) {
            if (typeof data.importerModuleId !== "string") {
              const err36 = { instancePath: instancePath + "/importerModuleId", schemaPath: "#/oneOf/4/properties/importerModuleId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
              if (vErrors === null) {
                vErrors = [err36];
              } else {
                vErrors.push(err36);
              }
              errors++;
            }
          }
          if (data.kind !== void 0) {
            let data12 = data.kind;
            if (typeof data12 !== "string") {
              const err37 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/4/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
              if (vErrors === null) {
                vErrors = [err37];
              } else {
                vErrors.push(err37);
              }
              errors++;
            }
            if ("import" !== data12) {
              const err38 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/4/properties/kind/const", keyword: "const", params: { allowedValue: "import" }, message: "must be equal to constant" };
              if (vErrors === null) {
                vErrors = [err38];
              } else {
                vErrors.push(err38);
              }
              errors++;
            }
          }
          if (data.localNamespaceId !== void 0) {
            if (typeof data.localNamespaceId !== "string") {
              const err39 = { instancePath: instancePath + "/localNamespaceId", schemaPath: "#/oneOf/4/properties/localNamespaceId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
              if (vErrors === null) {
                vErrors = [err39];
              } else {
                vErrors.push(err39);
              }
              errors++;
            }
          }
          if (data.sourceNamespaceId !== void 0) {
            if (typeof data.sourceNamespaceId !== "string") {
              const err40 = { instancePath: instancePath + "/sourceNamespaceId", schemaPath: "#/oneOf/4/properties/sourceNamespaceId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
              if (vErrors === null) {
                vErrors = [err40];
              } else {
                vErrors.push(err40);
              }
              errors++;
            }
          }
          if (data.upstreamModuleId !== void 0) {
            if (typeof data.upstreamModuleId !== "string") {
              const err41 = { instancePath: instancePath + "/upstreamModuleId", schemaPath: "#/oneOf/4/properties/upstreamModuleId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
              if (vErrors === null) {
                vErrors = [err41];
              } else {
                vErrors.push(err41);
              }
              errors++;
            }
          }
          if (data.upstreamReleaseId !== void 0) {
            if (typeof data.upstreamReleaseId !== "string") {
              const err42 = { instancePath: instancePath + "/upstreamReleaseId", schemaPath: "#/oneOf/4/properties/upstreamReleaseId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
              if (vErrors === null) {
                vErrors = [err42];
              } else {
                vErrors.push(err42);
              }
              errors++;
            }
          }
        } else {
          const err43 = { instancePath, schemaPath: "#/oneOf/4/type", keyword: "type", params: { type: "object" }, message: "must be object" };
          if (vErrors === null) {
            vErrors = [err43];
          } else {
            vErrors.push(err43);
          }
          errors++;
        }
        var _valid0 = _errs29 === errors;
        if (_valid0 && valid0) {
          valid0 = false;
          passing0 = [passing0, 4];
        } else {
          if (_valid0) {
            valid0 = true;
            passing0 = 4;
            if (props0 !== true) {
              props0 = props0 || {};
              props0.exportId = true;
              props0.importerModuleId = true;
              props0.kind = true;
              props0.localNamespaceId = true;
              props0.sourceNamespaceId = true;
              props0.upstreamModuleId = true;
              props0.upstreamReleaseId = true;
            }
          }
          const _errs45 = errors;
          if (data && typeof data == "object" && !Array.isArray(data)) {
            if (data.kind === void 0) {
              const err44 = { instancePath, schemaPath: "#/oneOf/5/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
              if (vErrors === null) {
                vErrors = [err44];
              } else {
                vErrors.push(err44);
              }
              errors++;
            }
            if (data.ownerModuleId === void 0) {
              const err45 = { instancePath, schemaPath: "#/oneOf/5/required", keyword: "required", params: { missingProperty: "ownerModuleId" }, message: "must have required property 'ownerModuleId'" };
              if (vErrors === null) {
                vErrors = [err45];
              } else {
                vErrors.push(err45);
              }
              errors++;
            }
            if (data.extensionNamespaceId === void 0) {
              const err46 = { instancePath, schemaPath: "#/oneOf/5/required", keyword: "required", params: { missingProperty: "extensionNamespaceId" }, message: "must have required property 'extensionNamespaceId'" };
              if (vErrors === null) {
                vErrors = [err46];
              } else {
                vErrors.push(err46);
              }
              errors++;
            }
            if (data.targetNamespaceId === void 0) {
              const err47 = { instancePath, schemaPath: "#/oneOf/5/required", keyword: "required", params: { missingProperty: "targetNamespaceId" }, message: "must have required property 'targetNamespaceId'" };
              if (vErrors === null) {
                vErrors = [err47];
              } else {
                vErrors.push(err47);
              }
              errors++;
            }
            if (data.targetDeclarationId === void 0) {
              const err48 = { instancePath, schemaPath: "#/oneOf/5/required", keyword: "required", params: { missingProperty: "targetDeclarationId" }, message: "must have required property 'targetDeclarationId'" };
              if (vErrors === null) {
                vErrors = [err48];
              } else {
                vErrors.push(err48);
              }
              errors++;
            }
            if (data.extensionNamespaceId !== void 0) {
              if (typeof data.extensionNamespaceId !== "string") {
                const err49 = { instancePath: instancePath + "/extensionNamespaceId", schemaPath: "#/oneOf/5/properties/extensionNamespaceId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                if (vErrors === null) {
                  vErrors = [err49];
                } else {
                  vErrors.push(err49);
                }
                errors++;
              }
            }
            if (data.kind !== void 0) {
              let data18 = data.kind;
              if (typeof data18 !== "string") {
                const err50 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/5/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                if (vErrors === null) {
                  vErrors = [err50];
                } else {
                  vErrors.push(err50);
                }
                errors++;
              }
              if ("extension" !== data18) {
                const err51 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/5/properties/kind/const", keyword: "const", params: { allowedValue: "extension" }, message: "must be equal to constant" };
                if (vErrors === null) {
                  vErrors = [err51];
                } else {
                  vErrors.push(err51);
                }
                errors++;
              }
            }
            if (data.ownerModuleId !== void 0) {
              if (typeof data.ownerModuleId !== "string") {
                const err52 = { instancePath: instancePath + "/ownerModuleId", schemaPath: "#/oneOf/5/properties/ownerModuleId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                if (vErrors === null) {
                  vErrors = [err52];
                } else {
                  vErrors.push(err52);
                }
                errors++;
              }
            }
            if (data.targetDeclarationId !== void 0) {
              if (typeof data.targetDeclarationId !== "string") {
                const err53 = { instancePath: instancePath + "/targetDeclarationId", schemaPath: "#/oneOf/5/properties/targetDeclarationId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                if (vErrors === null) {
                  vErrors = [err53];
                } else {
                  vErrors.push(err53);
                }
                errors++;
              }
            }
            if (data.targetNamespaceId !== void 0) {
              if (typeof data.targetNamespaceId !== "string") {
                const err54 = { instancePath: instancePath + "/targetNamespaceId", schemaPath: "#/oneOf/5/properties/targetNamespaceId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                if (vErrors === null) {
                  vErrors = [err54];
                } else {
                  vErrors.push(err54);
                }
                errors++;
              }
            }
          } else {
            const err55 = { instancePath, schemaPath: "#/oneOf/5/type", keyword: "type", params: { type: "object" }, message: "must be object" };
            if (vErrors === null) {
              vErrors = [err55];
            } else {
              vErrors.push(err55);
            }
            errors++;
          }
          var _valid0 = _errs45 === errors;
          if (_valid0 && valid0) {
            valid0 = false;
            passing0 = [passing0, 5];
          } else {
            if (_valid0) {
              valid0 = true;
              passing0 = 5;
              if (props0 !== true) {
                props0 = props0 || {};
                props0.extensionNamespaceId = true;
                props0.kind = true;
                props0.ownerModuleId = true;
                props0.targetDeclarationId = true;
                props0.targetNamespaceId = true;
              }
            }
            const _errs57 = errors;
            if (data && typeof data == "object" && !Array.isArray(data)) {
              if (data.kind === void 0) {
                const err56 = { instancePath, schemaPath: "#/oneOf/6/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
                if (vErrors === null) {
                  vErrors = [err56];
                } else {
                  vErrors.push(err56);
                }
                errors++;
              }
              if (data.ownerModuleId === void 0) {
                const err57 = { instancePath, schemaPath: "#/oneOf/6/required", keyword: "required", params: { missingProperty: "ownerModuleId" }, message: "must have required property 'ownerModuleId'" };
                if (vErrors === null) {
                  vErrors = [err57];
                } else {
                  vErrors.push(err57);
                }
                errors++;
              }
              if (data.namespaceId === void 0) {
                const err58 = { instancePath, schemaPath: "#/oneOf/6/required", keyword: "required", params: { missingProperty: "namespaceId" }, message: "must have required property 'namespaceId'" };
                if (vErrors === null) {
                  vErrors = [err58];
                } else {
                  vErrors.push(err58);
                }
                errors++;
              }
              if (data.declarationId === void 0) {
                const err59 = { instancePath, schemaPath: "#/oneOf/6/required", keyword: "required", params: { missingProperty: "declarationId" }, message: "must have required property 'declarationId'" };
                if (vErrors === null) {
                  vErrors = [err59];
                } else {
                  vErrors.push(err59);
                }
                errors++;
              }
              if (data.declarationId !== void 0) {
                if (typeof data.declarationId !== "string") {
                  const err60 = { instancePath: instancePath + "/declarationId", schemaPath: "#/oneOf/6/properties/declarationId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                  if (vErrors === null) {
                    vErrors = [err60];
                  } else {
                    vErrors.push(err60);
                  }
                  errors++;
                }
              }
              if (data.kind !== void 0) {
                let data23 = data.kind;
                if (typeof data23 !== "string") {
                  const err61 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/6/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                  if (vErrors === null) {
                    vErrors = [err61];
                  } else {
                    vErrors.push(err61);
                  }
                  errors++;
                }
                if ("policy" !== data23) {
                  const err62 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/6/properties/kind/const", keyword: "const", params: { allowedValue: "policy" }, message: "must be equal to constant" };
                  if (vErrors === null) {
                    vErrors = [err62];
                  } else {
                    vErrors.push(err62);
                  }
                  errors++;
                }
              }
              if (data.namespaceId !== void 0) {
                if (typeof data.namespaceId !== "string") {
                  const err63 = { instancePath: instancePath + "/namespaceId", schemaPath: "#/oneOf/6/properties/namespaceId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                  if (vErrors === null) {
                    vErrors = [err63];
                  } else {
                    vErrors.push(err63);
                  }
                  errors++;
                }
              }
              if (data.ownerModuleId !== void 0) {
                if (typeof data.ownerModuleId !== "string") {
                  const err64 = { instancePath: instancePath + "/ownerModuleId", schemaPath: "#/oneOf/6/properties/ownerModuleId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                  if (vErrors === null) {
                    vErrors = [err64];
                  } else {
                    vErrors.push(err64);
                  }
                  errors++;
                }
              }
            } else {
              const err65 = { instancePath, schemaPath: "#/oneOf/6/type", keyword: "type", params: { type: "object" }, message: "must be object" };
              if (vErrors === null) {
                vErrors = [err65];
              } else {
                vErrors.push(err65);
              }
              errors++;
            }
            var _valid0 = _errs57 === errors;
            if (_valid0 && valid0) {
              valid0 = false;
              passing0 = [passing0, 6];
            } else {
              if (_valid0) {
                valid0 = true;
                passing0 = 6;
                if (props0 !== true) {
                  props0 = props0 || {};
                  props0.declarationId = true;
                  props0.kind = true;
                  props0.namespaceId = true;
                  props0.ownerModuleId = true;
                }
              }
              const _errs67 = errors;
              if (data && typeof data == "object" && !Array.isArray(data)) {
                if (data.kind === void 0) {
                  const err66 = { instancePath, schemaPath: "#/oneOf/7/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
                  if (vErrors === null) {
                    vErrors = [err66];
                  } else {
                    vErrors.push(err66);
                  }
                  errors++;
                }
                if (data.kind !== void 0) {
                  let data26 = data.kind;
                  if (typeof data26 !== "string") {
                    const err67 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/7/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                    if (vErrors === null) {
                      vErrors = [err67];
                    } else {
                      vErrors.push(err67);
                    }
                    errors++;
                  }
                  if ("responsibility-closure" !== data26) {
                    const err68 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/7/properties/kind/const", keyword: "const", params: { allowedValue: "responsibility-closure" }, message: "must be equal to constant" };
                    if (vErrors === null) {
                      vErrors = [err68];
                    } else {
                      vErrors.push(err68);
                    }
                    errors++;
                  }
                }
              } else {
                const err69 = { instancePath, schemaPath: "#/oneOf/7/type", keyword: "type", params: { type: "object" }, message: "must be object" };
                if (vErrors === null) {
                  vErrors = [err69];
                } else {
                  vErrors.push(err69);
                }
                errors++;
              }
              var _valid0 = _errs67 === errors;
              if (_valid0 && valid0) {
                valid0 = false;
                passing0 = [passing0, 7];
              } else {
                if (_valid0) {
                  valid0 = true;
                  passing0 = 7;
                  if (props0 !== true) {
                    props0 = props0 || {};
                    props0.kind = true;
                  }
                }
                const _errs71 = errors;
                if (data && typeof data == "object" && !Array.isArray(data)) {
                  if (data.kind === void 0) {
                    const err70 = { instancePath, schemaPath: "#/oneOf/8/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
                    if (vErrors === null) {
                      vErrors = [err70];
                    } else {
                      vErrors.push(err70);
                    }
                    errors++;
                  }
                  if (data.kind !== void 0) {
                    let data27 = data.kind;
                    if (typeof data27 !== "string") {
                      const err71 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/8/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                      if (vErrors === null) {
                        vErrors = [err71];
                      } else {
                        vErrors.push(err71);
                      }
                      errors++;
                    }
                    if ("organizational-translation-closure" !== data27) {
                      const err72 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/8/properties/kind/const", keyword: "const", params: { allowedValue: "organizational-translation-closure" }, message: "must be equal to constant" };
                      if (vErrors === null) {
                        vErrors = [err72];
                      } else {
                        vErrors.push(err72);
                      }
                      errors++;
                    }
                  }
                } else {
                  const err73 = { instancePath, schemaPath: "#/oneOf/8/type", keyword: "type", params: { type: "object" }, message: "must be object" };
                  if (vErrors === null) {
                    vErrors = [err73];
                  } else {
                    vErrors.push(err73);
                  }
                  errors++;
                }
                var _valid0 = _errs71 === errors;
                if (_valid0 && valid0) {
                  valid0 = false;
                  passing0 = [passing0, 8];
                } else {
                  if (_valid0) {
                    valid0 = true;
                    passing0 = 8;
                    if (props0 !== true) {
                      props0 = props0 || {};
                      props0.kind = true;
                    }
                  }
                  const _errs75 = errors;
                  if (data && typeof data == "object" && !Array.isArray(data)) {
                    if (data.kind === void 0) {
                      const err74 = { instancePath, schemaPath: "#/oneOf/9/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
                      if (vErrors === null) {
                        vErrors = [err74];
                      } else {
                        vErrors.push(err74);
                      }
                      errors++;
                    }
                    if (data.kind !== void 0) {
                      let data28 = data.kind;
                      if (typeof data28 !== "string") {
                        const err75 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/9/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                        if (vErrors === null) {
                          vErrors = [err75];
                        } else {
                          vErrors.push(err75);
                        }
                        errors++;
                      }
                      if ("experience-composition" !== data28) {
                        const err76 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/9/properties/kind/const", keyword: "const", params: { allowedValue: "experience-composition" }, message: "must be equal to constant" };
                        if (vErrors === null) {
                          vErrors = [err76];
                        } else {
                          vErrors.push(err76);
                        }
                        errors++;
                      }
                    }
                  } else {
                    const err77 = { instancePath, schemaPath: "#/oneOf/9/type", keyword: "type", params: { type: "object" }, message: "must be object" };
                    if (vErrors === null) {
                      vErrors = [err77];
                    } else {
                      vErrors.push(err77);
                    }
                    errors++;
                  }
                  var _valid0 = _errs75 === errors;
                  if (_valid0 && valid0) {
                    valid0 = false;
                    passing0 = [passing0, 9];
                  } else {
                    if (_valid0) {
                      valid0 = true;
                      passing0 = 9;
                      if (props0 !== true) {
                        props0 = props0 || {};
                        props0.kind = true;
                      }
                    }
                    const _errs79 = errors;
                    if (data && typeof data == "object" && !Array.isArray(data)) {
                      if (data.kind === void 0) {
                        const err78 = { instancePath, schemaPath: "#/oneOf/10/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
                        if (vErrors === null) {
                          vErrors = [err78];
                        } else {
                          vErrors.push(err78);
                        }
                        errors++;
                      }
                      if (data.ownerModuleId === void 0) {
                        const err79 = { instancePath, schemaPath: "#/oneOf/10/required", keyword: "required", params: { missingProperty: "ownerModuleId" }, message: "must have required property 'ownerModuleId'" };
                        if (vErrors === null) {
                          vErrors = [err79];
                        } else {
                          vErrors.push(err79);
                        }
                        errors++;
                      }
                      if (data.resourceId === void 0) {
                        const err80 = { instancePath, schemaPath: "#/oneOf/10/required", keyword: "required", params: { missingProperty: "resourceId" }, message: "must have required property 'resourceId'" };
                        if (vErrors === null) {
                          vErrors = [err80];
                        } else {
                          vErrors.push(err80);
                        }
                        errors++;
                      }
                      if (data.kind !== void 0) {
                        let data29 = data.kind;
                        if (typeof data29 !== "string") {
                          const err81 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/10/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                          if (vErrors === null) {
                            vErrors = [err81];
                          } else {
                            vErrors.push(err81);
                          }
                          errors++;
                        }
                        if ("resource" !== data29) {
                          const err82 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/10/properties/kind/const", keyword: "const", params: { allowedValue: "resource" }, message: "must be equal to constant" };
                          if (vErrors === null) {
                            vErrors = [err82];
                          } else {
                            vErrors.push(err82);
                          }
                          errors++;
                        }
                      }
                      if (data.ownerModuleId !== void 0) {
                        if (typeof data.ownerModuleId !== "string") {
                          const err83 = { instancePath: instancePath + "/ownerModuleId", schemaPath: "#/oneOf/10/properties/ownerModuleId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                          if (vErrors === null) {
                            vErrors = [err83];
                          } else {
                            vErrors.push(err83);
                          }
                          errors++;
                        }
                      }
                      if (data.resourceId !== void 0) {
                        if (typeof data.resourceId !== "string") {
                          const err84 = { instancePath: instancePath + "/resourceId", schemaPath: "#/oneOf/10/properties/resourceId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                          if (vErrors === null) {
                            vErrors = [err84];
                          } else {
                            vErrors.push(err84);
                          }
                          errors++;
                        }
                      }
                    } else {
                      const err85 = { instancePath, schemaPath: "#/oneOf/10/type", keyword: "type", params: { type: "object" }, message: "must be object" };
                      if (vErrors === null) {
                        vErrors = [err85];
                      } else {
                        vErrors.push(err85);
                      }
                      errors++;
                    }
                    var _valid0 = _errs79 === errors;
                    if (_valid0 && valid0) {
                      valid0 = false;
                      passing0 = [passing0, 10];
                    } else {
                      if (_valid0) {
                        valid0 = true;
                        passing0 = 10;
                        if (props0 !== true) {
                          props0 = props0 || {};
                          props0.kind = true;
                          props0.ownerModuleId = true;
                          props0.resourceId = true;
                        }
                      }
                      const _errs87 = errors;
                      if (data && typeof data == "object" && !Array.isArray(data)) {
                        if (data.kind === void 0) {
                          const err86 = { instancePath, schemaPath: "#/oneOf/11/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
                          if (vErrors === null) {
                            vErrors = [err86];
                          } else {
                            vErrors.push(err86);
                          }
                          errors++;
                        }
                        if (data.kind !== void 0) {
                          let data32 = data.kind;
                          if (typeof data32 !== "string") {
                            const err87 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/11/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                            if (vErrors === null) {
                              vErrors = [err87];
                            } else {
                              vErrors.push(err87);
                            }
                            errors++;
                          }
                          if ("resolution-lock" !== data32) {
                            const err88 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/11/properties/kind/const", keyword: "const", params: { allowedValue: "resolution-lock" }, message: "must be equal to constant" };
                            if (vErrors === null) {
                              vErrors = [err88];
                            } else {
                              vErrors.push(err88);
                            }
                            errors++;
                          }
                        }
                      } else {
                        const err89 = { instancePath, schemaPath: "#/oneOf/11/type", keyword: "type", params: { type: "object" }, message: "must be object" };
                        if (vErrors === null) {
                          vErrors = [err89];
                        } else {
                          vErrors.push(err89);
                        }
                        errors++;
                      }
                      var _valid0 = _errs87 === errors;
                      if (_valid0 && valid0) {
                        valid0 = false;
                        passing0 = [passing0, 11];
                      } else {
                        if (_valid0) {
                          valid0 = true;
                          passing0 = 11;
                          if (props0 !== true) {
                            props0 = props0 || {};
                            props0.kind = true;
                          }
                        }
                        const _errs91 = errors;
                        if (data && typeof data == "object" && !Array.isArray(data)) {
                          if (data.kind === void 0) {
                            const err90 = { instancePath, schemaPath: "#/oneOf/12/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
                            if (vErrors === null) {
                              vErrors = [err90];
                            } else {
                              vErrors.push(err90);
                            }
                            errors++;
                          }
                          if (data.kind !== void 0) {
                            let data33 = data.kind;
                            if (typeof data33 !== "string") {
                              const err91 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/12/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                              if (vErrors === null) {
                                vErrors = [err91];
                              } else {
                                vErrors.push(err91);
                              }
                              errors++;
                            }
                            if ("universe-lock" !== data33) {
                              const err92 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/12/properties/kind/const", keyword: "const", params: { allowedValue: "universe-lock" }, message: "must be equal to constant" };
                              if (vErrors === null) {
                                vErrors = [err92];
                              } else {
                                vErrors.push(err92);
                              }
                              errors++;
                            }
                          }
                        } else {
                          const err93 = { instancePath, schemaPath: "#/oneOf/12/type", keyword: "type", params: { type: "object" }, message: "must be object" };
                          if (vErrors === null) {
                            vErrors = [err93];
                          } else {
                            vErrors.push(err93);
                          }
                          errors++;
                        }
                        var _valid0 = _errs91 === errors;
                        if (_valid0 && valid0) {
                          valid0 = false;
                          passing0 = [passing0, 12];
                        } else {
                          if (_valid0) {
                            valid0 = true;
                            passing0 = 12;
                            if (props0 !== true) {
                              props0 = props0 || {};
                              props0.kind = true;
                            }
                          }
                          const _errs95 = errors;
                          if (data && typeof data == "object" && !Array.isArray(data)) {
                            if (data.kind === void 0) {
                              const err94 = { instancePath, schemaPath: "#/oneOf/13/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
                              if (vErrors === null) {
                                vErrors = [err94];
                              } else {
                                vErrors.push(err94);
                              }
                              errors++;
                            }
                            if (data.workspaceId === void 0) {
                              const err95 = { instancePath, schemaPath: "#/oneOf/13/required", keyword: "required", params: { missingProperty: "workspaceId" }, message: "must have required property 'workspaceId'" };
                              if (vErrors === null) {
                                vErrors = [err95];
                              } else {
                                vErrors.push(err95);
                              }
                              errors++;
                            }
                            if (data.kind !== void 0) {
                              let data34 = data.kind;
                              if (typeof data34 !== "string") {
                                const err96 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/13/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                                if (vErrors === null) {
                                  vErrors = [err96];
                                } else {
                                  vErrors.push(err96);
                                }
                                errors++;
                              }
                              if ("workspace" !== data34) {
                                const err97 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/13/properties/kind/const", keyword: "const", params: { allowedValue: "workspace" }, message: "must be equal to constant" };
                                if (vErrors === null) {
                                  vErrors = [err97];
                                } else {
                                  vErrors.push(err97);
                                }
                                errors++;
                              }
                            }
                            if (data.workspaceId !== void 0) {
                              if (typeof data.workspaceId !== "string") {
                                const err98 = { instancePath: instancePath + "/workspaceId", schemaPath: "#/oneOf/13/properties/workspaceId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                                if (vErrors === null) {
                                  vErrors = [err98];
                                } else {
                                  vErrors.push(err98);
                                }
                                errors++;
                              }
                            }
                          } else {
                            const err99 = { instancePath, schemaPath: "#/oneOf/13/type", keyword: "type", params: { type: "object" }, message: "must be object" };
                            if (vErrors === null) {
                              vErrors = [err99];
                            } else {
                              vErrors.push(err99);
                            }
                            errors++;
                          }
                          var _valid0 = _errs95 === errors;
                          if (_valid0 && valid0) {
                            valid0 = false;
                            passing0 = [passing0, 13];
                          } else {
                            if (_valid0) {
                              valid0 = true;
                              passing0 = 13;
                              if (props0 !== true) {
                                props0 = props0 || {};
                                props0.kind = true;
                                props0.workspaceId = true;
                              }
                            }
                          }
                        }
                      }
                    }
                  }
                }
              }
            }
          }
        }
      }
    }
  }
  if (!valid0) {
    const err100 = { instancePath, schemaPath: "#/oneOf", keyword: "oneOf", params: { passingSchemas: passing0 }, message: "must match exactly one schema in oneOf" };
    if (vErrors === null) {
      vErrors = [err100];
    } else {
      vErrors.push(err100);
    }
    errors++;
  } else {
    errors = _errs0;
    if (vErrors !== null) {
      if (_errs0) {
        vErrors.length = _errs0;
      } else {
        vErrors = null;
      }
    }
  }
  validate203.errors = vErrors;
  evaluated0.props = props0;
  return errors === 0;
}
validate203.evaluated = { "dynamicProps": true, "dynamicItems": false };
function validate210(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate210.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  const _errs0 = errors;
  let valid0 = false;
  let passing0 = null;
  const _errs1 = errors;
  if (typeof data !== "string") {
    const err0 = { instancePath, schemaPath: "#/oneOf/0/type", keyword: "type", params: { type: "string" }, message: "must be string" };
    if (vErrors === null) {
      vErrors = [err0];
    } else {
      vErrors.push(err0);
    }
    errors++;
  }
  if ("read" !== data) {
    const err1 = { instancePath, schemaPath: "#/oneOf/0/const", keyword: "const", params: { allowedValue: "read" }, message: "must be equal to constant" };
    if (vErrors === null) {
      vErrors = [err1];
    } else {
      vErrors.push(err1);
    }
    errors++;
  }
  var _valid0 = _errs1 === errors;
  if (_valid0) {
    valid0 = true;
    passing0 = 0;
  }
  const _errs3 = errors;
  if (typeof data !== "string") {
    const err2 = { instancePath, schemaPath: "#/oneOf/1/type", keyword: "type", params: { type: "string" }, message: "must be string" };
    if (vErrors === null) {
      vErrors = [err2];
    } else {
      vErrors.push(err2);
    }
    errors++;
  }
  if ("mutate" !== data) {
    const err3 = { instancePath, schemaPath: "#/oneOf/1/const", keyword: "const", params: { allowedValue: "mutate" }, message: "must be equal to constant" };
    if (vErrors === null) {
      vErrors = [err3];
    } else {
      vErrors.push(err3);
    }
    errors++;
  }
  var _valid0 = _errs3 === errors;
  if (_valid0 && valid0) {
    valid0 = false;
    passing0 = [passing0, 1];
  } else {
    if (_valid0) {
      valid0 = true;
      passing0 = 1;
    }
  }
  if (!valid0) {
    const err4 = { instancePath, schemaPath: "#/oneOf", keyword: "oneOf", params: { passingSchemas: passing0 }, message: "must match exactly one schema in oneOf" };
    if (vErrors === null) {
      vErrors = [err4];
    } else {
      vErrors.push(err4);
    }
    errors++;
  } else {
    errors = _errs0;
    if (vErrors !== null) {
      if (_errs0) {
        vErrors.length = _errs0;
      } else {
        vErrors = null;
      }
    }
  }
  validate210.errors = vErrors;
  return errors === 0;
}
validate210.evaluated = { "dynamicProps": false, "dynamicItems": false };
function validate217(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate217.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.providerBindingId === void 0) {
      const err0 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "providerBindingId" }, message: "must have required property 'providerBindingId'" };
      if (vErrors === null) {
        vErrors = [err0];
      } else {
        vErrors.push(err0);
      }
      errors++;
    }
    if (data.providerBindingIdentity === void 0) {
      const err1 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "providerBindingIdentity" }, message: "must have required property 'providerBindingIdentity'" };
      if (vErrors === null) {
        vErrors = [err1];
      } else {
        vErrors.push(err1);
      }
      errors++;
    }
    for (const key0 in data) {
      if (!(key0 === "providerBindingId" || key0 === "providerBindingIdentity")) {
        const err2 = { instancePath, schemaPath: "#/additionalProperties", keyword: "additionalProperties", params: { additionalProperty: key0 }, message: "must NOT have additional properties" };
        if (vErrors === null) {
          vErrors = [err2];
        } else {
          vErrors.push(err2);
        }
        errors++;
      }
    }
    if (data.providerBindingId !== void 0) {
      if (typeof data.providerBindingId !== "string") {
        const err3 = { instancePath: instancePath + "/providerBindingId", schemaPath: "#/properties/providerBindingId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err3];
        } else {
          vErrors.push(err3);
        }
        errors++;
      }
    }
    if (data.providerBindingIdentity !== void 0) {
      if (typeof data.providerBindingIdentity !== "string") {
        const err4 = { instancePath: instancePath + "/providerBindingIdentity", schemaPath: "#/properties/providerBindingIdentity/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err4];
        } else {
          vErrors.push(err4);
        }
        errors++;
      }
    }
  } else {
    const err5 = { instancePath, schemaPath: "#/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err5];
    } else {
      vErrors.push(err5);
    }
    errors++;
  }
  validate217.errors = vErrors;
  return errors === 0;
}
validate217.evaluated = { "props": true, "dynamicProps": false, "dynamicItems": false };
function validate194(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate194.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  const _errs1 = errors;
  let valid0 = false;
  let passing0 = null;
  const _errs2 = errors;
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.kind === void 0) {
      const err0 = { instancePath, schemaPath: "#/oneOf/0/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
      if (vErrors === null) {
        vErrors = [err0];
      } else {
        vErrors.push(err0);
      }
      errors++;
    }
    if (data.selection === void 0) {
      const err1 = { instancePath, schemaPath: "#/oneOf/0/required", keyword: "required", params: { missingProperty: "selection" }, message: "must have required property 'selection'" };
      if (vErrors === null) {
        vErrors = [err1];
      } else {
        vErrors.push(err1);
      }
      errors++;
    }
    if (data.kind !== void 0) {
      let data0 = data.kind;
      if (typeof data0 !== "string") {
        const err2 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/0/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err2];
        } else {
          vErrors.push(err2);
        }
        errors++;
      }
      if ("declared-selection" !== data0) {
        const err3 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/0/properties/kind/const", keyword: "const", params: { allowedValue: "declared-selection" }, message: "must be equal to constant" };
        if (vErrors === null) {
          vErrors = [err3];
        } else {
          vErrors.push(err3);
        }
        errors++;
      }
    }
    if (data.selection !== void 0) {
      if (!validate149(data.selection, { instancePath: instancePath + "/selection", parentData: data, parentDataProperty: "selection", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate149.errors : vErrors.concat(validate149.errors);
        errors = vErrors.length;
      }
    }
  } else {
    const err4 = { instancePath, schemaPath: "#/oneOf/0/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err4];
    } else {
      vErrors.push(err4);
    }
    errors++;
  }
  var _valid0 = _errs2 === errors;
  if (_valid0) {
    valid0 = true;
    passing0 = 0;
    var props1 = {};
    props1.kind = true;
    props1.selection = true;
  }
  const _errs7 = errors;
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.kind === void 0) {
      const err5 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
      if (vErrors === null) {
        vErrors = [err5];
      } else {
        vErrors.push(err5);
      }
      errors++;
    }
    if (data.contractId === void 0) {
      const err6 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "contractId" }, message: "must have required property 'contractId'" };
      if (vErrors === null) {
        vErrors = [err6];
      } else {
        vErrors.push(err6);
      }
      errors++;
    }
    if (data.executionPathId === void 0) {
      const err7 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "executionPathId" }, message: "must have required property 'executionPathId'" };
      if (vErrors === null) {
        vErrors = [err7];
      } else {
        vErrors.push(err7);
      }
      errors++;
    }
    if (data.participant === void 0) {
      const err8 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "participant" }, message: "must have required property 'participant'" };
      if (vErrors === null) {
        vErrors = [err8];
      } else {
        vErrors.push(err8);
      }
      errors++;
    }
    if (data.responsibilities === void 0) {
      const err9 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "responsibilities" }, message: "must have required property 'responsibilities'" };
      if (vErrors === null) {
        vErrors = [err9];
      } else {
        vErrors.push(err9);
      }
      errors++;
    }
    if (data.authorityCeiling === void 0) {
      const err10 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "authorityCeiling" }, message: "must have required property 'authorityCeiling'" };
      if (vErrors === null) {
        vErrors = [err10];
      } else {
        vErrors.push(err10);
      }
      errors++;
    }
    if (data.reasonSteps === void 0) {
      const err11 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "reasonSteps" }, message: "must have required property 'reasonSteps'" };
      if (vErrors === null) {
        vErrors = [err11];
      } else {
        vErrors.push(err11);
      }
      errors++;
    }
    if (data.subject === void 0) {
      const err12 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "subject" }, message: "must have required property 'subject'" };
      if (vErrors === null) {
        vErrors = [err12];
      } else {
        vErrors.push(err12);
      }
      errors++;
    }
    if (data.authorityCeiling !== void 0) {
      let data2 = data.authorityCeiling;
      if (Array.isArray(data2)) {
        const len0 = data2.length;
        for (let i0 = 0; i0 < len0; i0++) {
          if (!validate196(data2[i0], { instancePath: instancePath + "/authorityCeiling/" + i0, parentData: data2, parentDataProperty: i0, rootData, dynamicAnchors })) {
            vErrors = vErrors === null ? validate196.errors : vErrors.concat(validate196.errors);
            errors = vErrors.length;
          }
        }
      } else {
        const err13 = { instancePath: instancePath + "/authorityCeiling", schemaPath: "#/oneOf/1/properties/authorityCeiling/type", keyword: "type", params: { type: "array" }, message: "must be array" };
        if (vErrors === null) {
          vErrors = [err13];
        } else {
          vErrors.push(err13);
        }
        errors++;
      }
    }
    if (data.contractId !== void 0) {
      if (typeof data.contractId !== "string") {
        const err14 = { instancePath: instancePath + "/contractId", schemaPath: "#/oneOf/1/properties/contractId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err14];
        } else {
          vErrors.push(err14);
        }
        errors++;
      }
    }
    if (data.executionPathId !== void 0) {
      if (typeof data.executionPathId !== "string") {
        const err15 = { instancePath: instancePath + "/executionPathId", schemaPath: "#/oneOf/1/properties/executionPathId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err15];
        } else {
          vErrors.push(err15);
        }
        errors++;
      }
    }
    if (data.kind !== void 0) {
      let data6 = data.kind;
      if (typeof data6 !== "string") {
        const err16 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/1/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err16];
        } else {
          vErrors.push(err16);
        }
        errors++;
      }
      if ("responsibility" !== data6) {
        const err17 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/1/properties/kind/const", keyword: "const", params: { allowedValue: "responsibility" }, message: "must be equal to constant" };
        if (vErrors === null) {
          vErrors = [err17];
        } else {
          vErrors.push(err17);
        }
        errors++;
      }
    }
    if (data.participant !== void 0) {
      if (!validate198(data.participant, { instancePath: instancePath + "/participant", parentData: data, parentDataProperty: "participant", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate198.errors : vErrors.concat(validate198.errors);
        errors = vErrors.length;
      }
    }
    if (data.reasonSteps !== void 0) {
      let data8 = data.reasonSteps;
      if (Array.isArray(data8)) {
        const len1 = data8.length;
        for (let i1 = 0; i1 < len1; i1++) {
          if (!validate157(data8[i1], { instancePath: instancePath + "/reasonSteps/" + i1, parentData: data8, parentDataProperty: i1, rootData, dynamicAnchors })) {
            vErrors = vErrors === null ? validate157.errors : vErrors.concat(validate157.errors);
            errors = vErrors.length;
          }
        }
      } else {
        const err18 = { instancePath: instancePath + "/reasonSteps", schemaPath: "#/oneOf/1/properties/reasonSteps/type", keyword: "type", params: { type: "array" }, message: "must be array" };
        if (vErrors === null) {
          vErrors = [err18];
        } else {
          vErrors.push(err18);
        }
        errors++;
      }
    }
    if (data.responsibilities !== void 0) {
      let data10 = data.responsibilities;
      if (Array.isArray(data10)) {
        const len2 = data10.length;
        for (let i2 = 0; i2 < len2; i2++) {
          if (!validate201(data10[i2], { instancePath: instancePath + "/responsibilities/" + i2, parentData: data10, parentDataProperty: i2, rootData, dynamicAnchors })) {
            vErrors = vErrors === null ? validate201.errors : vErrors.concat(validate201.errors);
            errors = vErrors.length;
          }
        }
      } else {
        const err19 = { instancePath: instancePath + "/responsibilities", schemaPath: "#/oneOf/1/properties/responsibilities/type", keyword: "type", params: { type: "array" }, message: "must be array" };
        if (vErrors === null) {
          vErrors = [err19];
        } else {
          vErrors.push(err19);
        }
        errors++;
      }
    }
    if (data.subject !== void 0) {
      if (!validate203(data.subject, { instancePath: instancePath + "/subject", parentData: data, parentDataProperty: "subject", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate203.errors : vErrors.concat(validate203.errors);
        errors = vErrors.length;
      }
    }
  } else {
    const err20 = { instancePath, schemaPath: "#/oneOf/1/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err20];
    } else {
      vErrors.push(err20);
    }
    errors++;
  }
  var _valid0 = _errs7 === errors;
  if (_valid0 && valid0) {
    valid0 = false;
    passing0 = [passing0, 1];
  } else {
    if (_valid0) {
      valid0 = true;
      passing0 = 1;
      if (props1 !== true) {
        props1 = props1 || {};
        props1.authorityCeiling = true;
        props1.contractId = true;
        props1.executionPathId = true;
        props1.kind = true;
        props1.participant = true;
        props1.reasonSteps = true;
        props1.responsibilities = true;
        props1.subject = true;
      }
    }
    const _errs26 = errors;
    if (data && typeof data == "object" && !Array.isArray(data)) {
      if (data.kind === void 0) {
        const err21 = { instancePath, schemaPath: "#/oneOf/2/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
        if (vErrors === null) {
          vErrors = [err21];
        } else {
          vErrors.push(err21);
        }
        errors++;
      }
      if (data.selection === void 0) {
        const err22 = { instancePath, schemaPath: "#/oneOf/2/required", keyword: "required", params: { missingProperty: "selection" }, message: "must have required property 'selection'" };
        if (vErrors === null) {
          vErrors = [err22];
        } else {
          vErrors.push(err22);
        }
        errors++;
      }
      if (data.kind !== void 0) {
        let data13 = data.kind;
        if (typeof data13 !== "string") {
          const err23 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/2/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
          if (vErrors === null) {
            vErrors = [err23];
          } else {
            vErrors.push(err23);
          }
          errors++;
        }
        if ("selection" !== data13) {
          const err24 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/2/properties/kind/const", keyword: "const", params: { allowedValue: "selection" }, message: "must be equal to constant" };
          if (vErrors === null) {
            vErrors = [err24];
          } else {
            vErrors.push(err24);
          }
          errors++;
        }
      }
      if (data.selection !== void 0) {
        if (!validate159(data.selection, { instancePath: instancePath + "/selection", parentData: data, parentDataProperty: "selection", rootData, dynamicAnchors })) {
          vErrors = vErrors === null ? validate159.errors : vErrors.concat(validate159.errors);
          errors = vErrors.length;
        }
      }
    } else {
      const err25 = { instancePath, schemaPath: "#/oneOf/2/type", keyword: "type", params: { type: "object" }, message: "must be object" };
      if (vErrors === null) {
        vErrors = [err25];
      } else {
        vErrors.push(err25);
      }
      errors++;
    }
    var _valid0 = _errs26 === errors;
    if (_valid0 && valid0) {
      valid0 = false;
      passing0 = [passing0, 2];
    } else {
      if (_valid0) {
        valid0 = true;
        passing0 = 2;
        if (props1 !== true) {
          props1 = props1 || {};
          props1.kind = true;
          props1.selection = true;
        }
      }
      const _errs31 = errors;
      if (data && typeof data == "object" && !Array.isArray(data)) {
        if (data.kind === void 0) {
          const err26 = { instancePath, schemaPath: "#/oneOf/3/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
          if (vErrors === null) {
            vErrors = [err26];
          } else {
            vErrors.push(err26);
          }
          errors++;
        }
        if (data.permittedCapabilities === void 0) {
          const err27 = { instancePath, schemaPath: "#/oneOf/3/required", keyword: "required", params: { missingProperty: "permittedCapabilities" }, message: "must have required property 'permittedCapabilities'" };
          if (vErrors === null) {
            vErrors = [err27];
          } else {
            vErrors.push(err27);
          }
          errors++;
        }
        if (data.forbiddenCapabilities === void 0) {
          const err28 = { instancePath, schemaPath: "#/oneOf/3/required", keyword: "required", params: { missingProperty: "forbiddenCapabilities" }, message: "must have required property 'forbiddenCapabilities'" };
          if (vErrors === null) {
            vErrors = [err28];
          } else {
            vErrors.push(err28);
          }
          errors++;
        }
        if (data.permittedEffects === void 0) {
          const err29 = { instancePath, schemaPath: "#/oneOf/3/required", keyword: "required", params: { missingProperty: "permittedEffects" }, message: "must have required property 'permittedEffects'" };
          if (vErrors === null) {
            vErrors = [err29];
          } else {
            vErrors.push(err29);
          }
          errors++;
        }
        if (data.forbiddenEffects === void 0) {
          const err30 = { instancePath, schemaPath: "#/oneOf/3/required", keyword: "required", params: { missingProperty: "forbiddenEffects" }, message: "must have required property 'forbiddenEffects'" };
          if (vErrors === null) {
            vErrors = [err30];
          } else {
            vErrors.push(err30);
          }
          errors++;
        }
        if (data.forbiddenCapabilities !== void 0) {
          let data15 = data.forbiddenCapabilities;
          if (Array.isArray(data15)) {
            const len3 = data15.length;
            for (let i3 = 0; i3 < len3; i3++) {
              if (typeof data15[i3] !== "string") {
                const err31 = { instancePath: instancePath + "/forbiddenCapabilities/" + i3, schemaPath: "#/oneOf/3/properties/forbiddenCapabilities/items/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                if (vErrors === null) {
                  vErrors = [err31];
                } else {
                  vErrors.push(err31);
                }
                errors++;
              }
            }
          } else {
            const err32 = { instancePath: instancePath + "/forbiddenCapabilities", schemaPath: "#/oneOf/3/properties/forbiddenCapabilities/type", keyword: "type", params: { type: "array" }, message: "must be array" };
            if (vErrors === null) {
              vErrors = [err32];
            } else {
              vErrors.push(err32);
            }
            errors++;
          }
        }
        if (data.forbiddenEffects !== void 0) {
          let data17 = data.forbiddenEffects;
          if (Array.isArray(data17)) {
            const len4 = data17.length;
            for (let i4 = 0; i4 < len4; i4++) {
              if (!validate196(data17[i4], { instancePath: instancePath + "/forbiddenEffects/" + i4, parentData: data17, parentDataProperty: i4, rootData, dynamicAnchors })) {
                vErrors = vErrors === null ? validate196.errors : vErrors.concat(validate196.errors);
                errors = vErrors.length;
              }
            }
          } else {
            const err33 = { instancePath: instancePath + "/forbiddenEffects", schemaPath: "#/oneOf/3/properties/forbiddenEffects/type", keyword: "type", params: { type: "array" }, message: "must be array" };
            if (vErrors === null) {
              vErrors = [err33];
            } else {
              vErrors.push(err33);
            }
            errors++;
          }
        }
        if (data.kind !== void 0) {
          let data19 = data.kind;
          if (typeof data19 !== "string") {
            const err34 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/3/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
            if (vErrors === null) {
              vErrors = [err34];
            } else {
              vErrors.push(err34);
            }
            errors++;
          }
          if ("capability-narrowing" !== data19) {
            const err35 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/3/properties/kind/const", keyword: "const", params: { allowedValue: "capability-narrowing" }, message: "must be equal to constant" };
            if (vErrors === null) {
              vErrors = [err35];
            } else {
              vErrors.push(err35);
            }
            errors++;
          }
        }
        if (data.permittedCapabilities !== void 0) {
          let data20 = data.permittedCapabilities;
          if (Array.isArray(data20)) {
            const len5 = data20.length;
            for (let i5 = 0; i5 < len5; i5++) {
              if (typeof data20[i5] !== "string") {
                const err36 = { instancePath: instancePath + "/permittedCapabilities/" + i5, schemaPath: "#/oneOf/3/properties/permittedCapabilities/items/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                if (vErrors === null) {
                  vErrors = [err36];
                } else {
                  vErrors.push(err36);
                }
                errors++;
              }
            }
          } else {
            const err37 = { instancePath: instancePath + "/permittedCapabilities", schemaPath: "#/oneOf/3/properties/permittedCapabilities/type", keyword: "type", params: { type: "array" }, message: "must be array" };
            if (vErrors === null) {
              vErrors = [err37];
            } else {
              vErrors.push(err37);
            }
            errors++;
          }
        }
        if (data.permittedEffects !== void 0) {
          let data22 = data.permittedEffects;
          if (Array.isArray(data22)) {
            const len6 = data22.length;
            for (let i6 = 0; i6 < len6; i6++) {
              if (!validate196(data22[i6], { instancePath: instancePath + "/permittedEffects/" + i6, parentData: data22, parentDataProperty: i6, rootData, dynamicAnchors })) {
                vErrors = vErrors === null ? validate196.errors : vErrors.concat(validate196.errors);
                errors = vErrors.length;
              }
            }
          } else {
            const err38 = { instancePath: instancePath + "/permittedEffects", schemaPath: "#/oneOf/3/properties/permittedEffects/type", keyword: "type", params: { type: "array" }, message: "must be array" };
            if (vErrors === null) {
              vErrors = [err38];
            } else {
              vErrors.push(err38);
            }
            errors++;
          }
        }
      } else {
        const err39 = { instancePath, schemaPath: "#/oneOf/3/type", keyword: "type", params: { type: "object" }, message: "must be object" };
        if (vErrors === null) {
          vErrors = [err39];
        } else {
          vErrors.push(err39);
        }
        errors++;
      }
      var _valid0 = _errs31 === errors;
      if (_valid0 && valid0) {
        valid0 = false;
        passing0 = [passing0, 3];
      } else {
        if (_valid0) {
          valid0 = true;
          passing0 = 3;
          if (props1 !== true) {
            props1 = props1 || {};
            props1.forbiddenCapabilities = true;
            props1.forbiddenEffects = true;
            props1.kind = true;
            props1.permittedCapabilities = true;
            props1.permittedEffects = true;
          }
        }
        const _errs49 = errors;
        if (data && typeof data == "object" && !Array.isArray(data)) {
          if (data.kind === void 0) {
            const err40 = { instancePath, schemaPath: "#/oneOf/4/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
            if (vErrors === null) {
              vErrors = [err40];
            } else {
              vErrors.push(err40);
            }
            errors++;
          }
          if (data.scope === void 0) {
            const err41 = { instancePath, schemaPath: "#/oneOf/4/required", keyword: "required", params: { missingProperty: "scope" }, message: "must have required property 'scope'" };
            if (vErrors === null) {
              vErrors = [err41];
            } else {
              vErrors.push(err41);
            }
            errors++;
          }
          if (data.subjectResource === void 0) {
            const err42 = { instancePath, schemaPath: "#/oneOf/4/required", keyword: "required", params: { missingProperty: "subjectResource" }, message: "must have required property 'subjectResource'" };
            if (vErrors === null) {
              vErrors = [err42];
            } else {
              vErrors.push(err42);
            }
            errors++;
          }
          if (data.contributors === void 0) {
            const err43 = { instancePath, schemaPath: "#/oneOf/4/required", keyword: "required", params: { missingProperty: "contributors" }, message: "must have required property 'contributors'" };
            if (vErrors === null) {
              vErrors = [err43];
            } else {
              vErrors.push(err43);
            }
            errors++;
          }
          if (data.policies === void 0) {
            const err44 = { instancePath, schemaPath: "#/oneOf/4/required", keyword: "required", params: { missingProperty: "policies" }, message: "must have required property 'policies'" };
            if (vErrors === null) {
              vErrors = [err44];
            } else {
              vErrors.push(err44);
            }
            errors++;
          }
          if (data.permittedCapabilities === void 0) {
            const err45 = { instancePath, schemaPath: "#/oneOf/4/required", keyword: "required", params: { missingProperty: "permittedCapabilities" }, message: "must have required property 'permittedCapabilities'" };
            if (vErrors === null) {
              vErrors = [err45];
            } else {
              vErrors.push(err45);
            }
            errors++;
          }
          if (data.forbiddenCapabilities === void 0) {
            const err46 = { instancePath, schemaPath: "#/oneOf/4/required", keyword: "required", params: { missingProperty: "forbiddenCapabilities" }, message: "must have required property 'forbiddenCapabilities'" };
            if (vErrors === null) {
              vErrors = [err46];
            } else {
              vErrors.push(err46);
            }
            errors++;
          }
          if (data.permittedEffects === void 0) {
            const err47 = { instancePath, schemaPath: "#/oneOf/4/required", keyword: "required", params: { missingProperty: "permittedEffects" }, message: "must have required property 'permittedEffects'" };
            if (vErrors === null) {
              vErrors = [err47];
            } else {
              vErrors.push(err47);
            }
            errors++;
          }
          if (data.forbiddenEffects === void 0) {
            const err48 = { instancePath, schemaPath: "#/oneOf/4/required", keyword: "required", params: { missingProperty: "forbiddenEffects" }, message: "must have required property 'forbiddenEffects'" };
            if (vErrors === null) {
              vErrors = [err48];
            } else {
              vErrors.push(err48);
            }
            errors++;
          }
          if (data.contributors !== void 0) {
            let data24 = data.contributors;
            if (Array.isArray(data24)) {
              const len7 = data24.length;
              for (let i7 = 0; i7 < len7; i7++) {
                if (typeof data24[i7] !== "string") {
                  const err49 = { instancePath: instancePath + "/contributors/" + i7, schemaPath: "#/oneOf/4/properties/contributors/items/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                  if (vErrors === null) {
                    vErrors = [err49];
                  } else {
                    vErrors.push(err49);
                  }
                  errors++;
                }
              }
            } else {
              const err50 = { instancePath: instancePath + "/contributors", schemaPath: "#/oneOf/4/properties/contributors/type", keyword: "type", params: { type: "array" }, message: "must be array" };
              if (vErrors === null) {
                vErrors = [err50];
              } else {
                vErrors.push(err50);
              }
              errors++;
            }
          }
          if (data.forbiddenCapabilities !== void 0) {
            let data26 = data.forbiddenCapabilities;
            if (Array.isArray(data26)) {
              const len8 = data26.length;
              for (let i8 = 0; i8 < len8; i8++) {
                if (typeof data26[i8] !== "string") {
                  const err51 = { instancePath: instancePath + "/forbiddenCapabilities/" + i8, schemaPath: "#/oneOf/4/properties/forbiddenCapabilities/items/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                  if (vErrors === null) {
                    vErrors = [err51];
                  } else {
                    vErrors.push(err51);
                  }
                  errors++;
                }
              }
            } else {
              const err52 = { instancePath: instancePath + "/forbiddenCapabilities", schemaPath: "#/oneOf/4/properties/forbiddenCapabilities/type", keyword: "type", params: { type: "array" }, message: "must be array" };
              if (vErrors === null) {
                vErrors = [err52];
              } else {
                vErrors.push(err52);
              }
              errors++;
            }
          }
          if (data.forbiddenEffects !== void 0) {
            let data28 = data.forbiddenEffects;
            if (Array.isArray(data28)) {
              const len9 = data28.length;
              for (let i9 = 0; i9 < len9; i9++) {
                if (!validate196(data28[i9], { instancePath: instancePath + "/forbiddenEffects/" + i9, parentData: data28, parentDataProperty: i9, rootData, dynamicAnchors })) {
                  vErrors = vErrors === null ? validate196.errors : vErrors.concat(validate196.errors);
                  errors = vErrors.length;
                }
              }
            } else {
              const err53 = { instancePath: instancePath + "/forbiddenEffects", schemaPath: "#/oneOf/4/properties/forbiddenEffects/type", keyword: "type", params: { type: "array" }, message: "must be array" };
              if (vErrors === null) {
                vErrors = [err53];
              } else {
                vErrors.push(err53);
              }
              errors++;
            }
          }
          if (data.kind !== void 0) {
            let data30 = data.kind;
            if (typeof data30 !== "string") {
              const err54 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/4/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
              if (vErrors === null) {
                vErrors = [err54];
              } else {
                vErrors.push(err54);
              }
              errors++;
            }
            if ("governing-ceiling" !== data30) {
              const err55 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/4/properties/kind/const", keyword: "const", params: { allowedValue: "governing-ceiling" }, message: "must be equal to constant" };
              if (vErrors === null) {
                vErrors = [err55];
              } else {
                vErrors.push(err55);
              }
              errors++;
            }
          }
          if (data.permittedCapabilities !== void 0) {
            let data31 = data.permittedCapabilities;
            if (Array.isArray(data31)) {
              const len10 = data31.length;
              for (let i10 = 0; i10 < len10; i10++) {
                if (typeof data31[i10] !== "string") {
                  const err56 = { instancePath: instancePath + "/permittedCapabilities/" + i10, schemaPath: "#/oneOf/4/properties/permittedCapabilities/items/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                  if (vErrors === null) {
                    vErrors = [err56];
                  } else {
                    vErrors.push(err56);
                  }
                  errors++;
                }
              }
            } else {
              const err57 = { instancePath: instancePath + "/permittedCapabilities", schemaPath: "#/oneOf/4/properties/permittedCapabilities/type", keyword: "type", params: { type: "array" }, message: "must be array" };
              if (vErrors === null) {
                vErrors = [err57];
              } else {
                vErrors.push(err57);
              }
              errors++;
            }
          }
          if (data.permittedEffects !== void 0) {
            let data33 = data.permittedEffects;
            if (Array.isArray(data33)) {
              const len11 = data33.length;
              for (let i11 = 0; i11 < len11; i11++) {
                if (!validate196(data33[i11], { instancePath: instancePath + "/permittedEffects/" + i11, parentData: data33, parentDataProperty: i11, rootData, dynamicAnchors })) {
                  vErrors = vErrors === null ? validate196.errors : vErrors.concat(validate196.errors);
                  errors = vErrors.length;
                }
              }
            } else {
              const err58 = { instancePath: instancePath + "/permittedEffects", schemaPath: "#/oneOf/4/properties/permittedEffects/type", keyword: "type", params: { type: "array" }, message: "must be array" };
              if (vErrors === null) {
                vErrors = [err58];
              } else {
                vErrors.push(err58);
              }
              errors++;
            }
          }
          if (data.policies !== void 0) {
            let data35 = data.policies;
            if (Array.isArray(data35)) {
              const len12 = data35.length;
              for (let i12 = 0; i12 < len12; i12++) {
                if (typeof data35[i12] !== "string") {
                  const err59 = { instancePath: instancePath + "/policies/" + i12, schemaPath: "#/oneOf/4/properties/policies/items/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                  if (vErrors === null) {
                    vErrors = [err59];
                  } else {
                    vErrors.push(err59);
                  }
                  errors++;
                }
              }
            } else {
              const err60 = { instancePath: instancePath + "/policies", schemaPath: "#/oneOf/4/properties/policies/type", keyword: "type", params: { type: "array" }, message: "must be array" };
              if (vErrors === null) {
                vErrors = [err60];
              } else {
                vErrors.push(err60);
              }
              errors++;
            }
          }
          if (data.scope !== void 0) {
            if (typeof data.scope !== "string") {
              const err61 = { instancePath: instancePath + "/scope", schemaPath: "#/oneOf/4/properties/scope/type", keyword: "type", params: { type: "string" }, message: "must be string" };
              if (vErrors === null) {
                vErrors = [err61];
              } else {
                vErrors.push(err61);
              }
              errors++;
            }
          }
          if (data.subjectResource !== void 0) {
            if (typeof data.subjectResource !== "string") {
              const err62 = { instancePath: instancePath + "/subjectResource", schemaPath: "#/oneOf/4/properties/subjectResource/type", keyword: "type", params: { type: "string" }, message: "must be string" };
              if (vErrors === null) {
                vErrors = [err62];
              } else {
                vErrors.push(err62);
              }
              errors++;
            }
          }
        } else {
          const err63 = { instancePath, schemaPath: "#/oneOf/4/type", keyword: "type", params: { type: "object" }, message: "must be object" };
          if (vErrors === null) {
            vErrors = [err63];
          } else {
            vErrors.push(err63);
          }
          errors++;
        }
        var _valid0 = _errs49 === errors;
        if (_valid0 && valid0) {
          valid0 = false;
          passing0 = [passing0, 4];
        } else {
          if (_valid0) {
            valid0 = true;
            passing0 = 4;
            if (props1 !== true) {
              props1 = props1 || {};
              props1.contributors = true;
              props1.forbiddenCapabilities = true;
              props1.forbiddenEffects = true;
              props1.kind = true;
              props1.permittedCapabilities = true;
              props1.permittedEffects = true;
              props1.policies = true;
              props1.scope = true;
              props1.subjectResource = true;
            }
          }
          const _errs79 = errors;
          if (data && typeof data == "object" && !Array.isArray(data)) {
            if (data.kind === void 0) {
              const err64 = { instancePath, schemaPath: "#/oneOf/5/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
              if (vErrors === null) {
                vErrors = [err64];
              } else {
                vErrors.push(err64);
              }
              errors++;
            }
            if (data.repositoryId === void 0) {
              const err65 = { instancePath, schemaPath: "#/oneOf/5/required", keyword: "required", params: { missingProperty: "repositoryId" }, message: "must have required property 'repositoryId'" };
              if (vErrors === null) {
                vErrors = [err65];
              } else {
                vErrors.push(err65);
              }
              errors++;
            }
            if (data.materializationOrder === void 0) {
              const err66 = { instancePath, schemaPath: "#/oneOf/5/required", keyword: "required", params: { missingProperty: "materializationOrder" }, message: "must have required property 'materializationOrder'" };
              if (vErrors === null) {
                vErrors = [err66];
              } else {
                vErrors.push(err66);
              }
              errors++;
            }
            if (data.accessCeiling === void 0) {
              const err67 = { instancePath, schemaPath: "#/oneOf/5/required", keyword: "required", params: { missingProperty: "accessCeiling" }, message: "must have required property 'accessCeiling'" };
              if (vErrors === null) {
                vErrors = [err67];
              } else {
                vErrors.push(err67);
              }
              errors++;
            }
            if (data.accessCeiling !== void 0) {
              if (!validate210(data.accessCeiling, { instancePath: instancePath + "/accessCeiling", parentData: data, parentDataProperty: "accessCeiling", rootData, dynamicAnchors })) {
                vErrors = vErrors === null ? validate210.errors : vErrors.concat(validate210.errors);
                errors = vErrors.length;
              }
            }
            if (data.kind !== void 0) {
              let data40 = data.kind;
              if (typeof data40 !== "string") {
                const err68 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/5/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                if (vErrors === null) {
                  vErrors = [err68];
                } else {
                  vErrors.push(err68);
                }
                errors++;
              }
              if ("repository" !== data40) {
                const err69 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/5/properties/kind/const", keyword: "const", params: { allowedValue: "repository" }, message: "must be equal to constant" };
                if (vErrors === null) {
                  vErrors = [err69];
                } else {
                  vErrors.push(err69);
                }
                errors++;
              }
            }
            if (data.materializationOrder !== void 0) {
              let data41 = data.materializationOrder;
              if (!(typeof data41 == "number" && (!(data41 % 1) && !isNaN(data41)))) {
                const err70 = { instancePath: instancePath + "/materializationOrder", schemaPath: "#/oneOf/5/properties/materializationOrder/type", keyword: "type", params: { type: "integer" }, message: "must be integer" };
                if (vErrors === null) {
                  vErrors = [err70];
                } else {
                  vErrors.push(err70);
                }
                errors++;
              }
              if (typeof data41 == "number") {
                if (data41 < 0 || isNaN(data41)) {
                  const err71 = { instancePath: instancePath + "/materializationOrder", schemaPath: "#/oneOf/5/properties/materializationOrder/minimum", keyword: "minimum", params: { comparison: ">=", limit: 0 }, message: "must be >= 0" };
                  if (vErrors === null) {
                    vErrors = [err71];
                  } else {
                    vErrors.push(err71);
                  }
                  errors++;
                }
                if (!formats4.validate(data41)) {
                  const err72 = { instancePath: instancePath + "/materializationOrder", schemaPath: "#/oneOf/5/properties/materializationOrder/format", keyword: "format", params: { format: "uint32" }, message: 'must match format "uint32"' };
                  if (vErrors === null) {
                    vErrors = [err72];
                  } else {
                    vErrors.push(err72);
                  }
                  errors++;
                }
              }
            }
            if (data.repositoryId !== void 0) {
              if (typeof data.repositoryId !== "string") {
                const err73 = { instancePath: instancePath + "/repositoryId", schemaPath: "#/oneOf/5/properties/repositoryId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                if (vErrors === null) {
                  vErrors = [err73];
                } else {
                  vErrors.push(err73);
                }
                errors++;
              }
            }
          } else {
            const err74 = { instancePath, schemaPath: "#/oneOf/5/type", keyword: "type", params: { type: "object" }, message: "must be object" };
            if (vErrors === null) {
              vErrors = [err74];
            } else {
              vErrors.push(err74);
            }
            errors++;
          }
          var _valid0 = _errs79 === errors;
          if (_valid0 && valid0) {
            valid0 = false;
            passing0 = [passing0, 5];
          } else {
            if (_valid0) {
              valid0 = true;
              passing0 = 5;
              if (props1 !== true) {
                props1 = props1 || {};
                props1.accessCeiling = true;
                props1.kind = true;
                props1.materializationOrder = true;
                props1.repositoryId = true;
              }
            }
            const _errs88 = errors;
            if (data && typeof data == "object" && !Array.isArray(data)) {
              if (data.kind === void 0) {
                const err75 = { instancePath, schemaPath: "#/oneOf/6/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
                if (vErrors === null) {
                  vErrors = [err75];
                } else {
                  vErrors.push(err75);
                }
                errors++;
              }
              if (data.repositoryId === void 0) {
                const err76 = { instancePath, schemaPath: "#/oneOf/6/required", keyword: "required", params: { missingProperty: "repositoryId" }, message: "must have required property 'repositoryId'" };
                if (vErrors === null) {
                  vErrors = [err76];
                } else {
                  vErrors.push(err76);
                }
                errors++;
              }
              if (data.providerBindingId === void 0) {
                const err77 = { instancePath, schemaPath: "#/oneOf/6/required", keyword: "required", params: { missingProperty: "providerBindingId" }, message: "must have required property 'providerBindingId'" };
                if (vErrors === null) {
                  vErrors = [err77];
                } else {
                  vErrors.push(err77);
                }
                errors++;
              }
              if (data.nativeRepositoryCoordinate === void 0) {
                const err78 = { instancePath, schemaPath: "#/oneOf/6/required", keyword: "required", params: { missingProperty: "nativeRepositoryCoordinate" }, message: "must have required property 'nativeRepositoryCoordinate'" };
                if (vErrors === null) {
                  vErrors = [err78];
                } else {
                  vErrors.push(err78);
                }
                errors++;
              }
              if (data.nativeRealization === void 0) {
                const err79 = { instancePath, schemaPath: "#/oneOf/6/required", keyword: "required", params: { missingProperty: "nativeRealization" }, message: "must have required property 'nativeRealization'" };
                if (vErrors === null) {
                  vErrors = [err79];
                } else {
                  vErrors.push(err79);
                }
                errors++;
              }
              if (data.kind !== void 0) {
                let data43 = data.kind;
                if (typeof data43 !== "string") {
                  const err80 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/6/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                  if (vErrors === null) {
                    vErrors = [err80];
                  } else {
                    vErrors.push(err80);
                  }
                  errors++;
                }
                if ("provider-repository" !== data43) {
                  const err81 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/6/properties/kind/const", keyword: "const", params: { allowedValue: "provider-repository" }, message: "must be equal to constant" };
                  if (vErrors === null) {
                    vErrors = [err81];
                  } else {
                    vErrors.push(err81);
                  }
                  errors++;
                }
              }
              if (data.nativeRealization !== void 0) {
                if (!validate165(data.nativeRealization, { instancePath: instancePath + "/nativeRealization", parentData: data, parentDataProperty: "nativeRealization", rootData, dynamicAnchors })) {
                  vErrors = vErrors === null ? validate165.errors : vErrors.concat(validate165.errors);
                  errors = vErrors.length;
                }
              }
              if (data.nativeRepositoryCoordinate !== void 0) {
                if (typeof data.nativeRepositoryCoordinate !== "string") {
                  const err82 = { instancePath: instancePath + "/nativeRepositoryCoordinate", schemaPath: "#/oneOf/6/properties/nativeRepositoryCoordinate/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                  if (vErrors === null) {
                    vErrors = [err82];
                  } else {
                    vErrors.push(err82);
                  }
                  errors++;
                }
              }
              if (data.providerBindingId !== void 0) {
                if (typeof data.providerBindingId !== "string") {
                  const err83 = { instancePath: instancePath + "/providerBindingId", schemaPath: "#/oneOf/6/properties/providerBindingId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                  if (vErrors === null) {
                    vErrors = [err83];
                  } else {
                    vErrors.push(err83);
                  }
                  errors++;
                }
              }
              if (data.repositoryId !== void 0) {
                if (typeof data.repositoryId !== "string") {
                  const err84 = { instancePath: instancePath + "/repositoryId", schemaPath: "#/oneOf/6/properties/repositoryId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                  if (vErrors === null) {
                    vErrors = [err84];
                  } else {
                    vErrors.push(err84);
                  }
                  errors++;
                }
              }
            } else {
              const err85 = { instancePath, schemaPath: "#/oneOf/6/type", keyword: "type", params: { type: "object" }, message: "must be object" };
              if (vErrors === null) {
                vErrors = [err85];
              } else {
                vErrors.push(err85);
              }
              errors++;
            }
            var _valid0 = _errs88 === errors;
            if (_valid0 && valid0) {
              valid0 = false;
              passing0 = [passing0, 6];
            } else {
              if (_valid0) {
                valid0 = true;
                passing0 = 6;
                if (props1 !== true) {
                  props1 = props1 || {};
                  props1.kind = true;
                  props1.nativeRealization = true;
                  props1.nativeRepositoryCoordinate = true;
                  props1.providerBindingId = true;
                  props1.repositoryId = true;
                }
              }
              const _errs99 = errors;
              if (data && typeof data == "object" && !Array.isArray(data)) {
                if (data.kind === void 0) {
                  const err86 = { instancePath, schemaPath: "#/oneOf/7/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
                  if (vErrors === null) {
                    vErrors = [err86];
                  } else {
                    vErrors.push(err86);
                  }
                  errors++;
                }
                if (data.selectionId === void 0) {
                  const err87 = { instancePath, schemaPath: "#/oneOf/7/required", keyword: "required", params: { missingProperty: "selectionId" }, message: "must have required property 'selectionId'" };
                  if (vErrors === null) {
                    vErrors = [err87];
                  } else {
                    vErrors.push(err87);
                  }
                  errors++;
                }
                if (data.experiencePathId === void 0) {
                  const err88 = { instancePath, schemaPath: "#/oneOf/7/required", keyword: "required", params: { missingProperty: "experiencePathId" }, message: "must have required property 'experiencePathId'" };
                  if (vErrors === null) {
                    vErrors = [err88];
                  } else {
                    vErrors.push(err88);
                  }
                  errors++;
                }
                if (data.responsibilityPathIdentity === void 0) {
                  const err89 = { instancePath, schemaPath: "#/oneOf/7/required", keyword: "required", params: { missingProperty: "responsibilityPathIdentity" }, message: "must have required property 'responsibilityPathIdentity'" };
                  if (vErrors === null) {
                    vErrors = [err89];
                  } else {
                    vErrors.push(err89);
                  }
                  errors++;
                }
                if (data.responsibilityContractIds === void 0) {
                  const err90 = { instancePath, schemaPath: "#/oneOf/7/required", keyword: "required", params: { missingProperty: "responsibilityContractIds" }, message: "must have required property 'responsibilityContractIds'" };
                  if (vErrors === null) {
                    vErrors = [err90];
                  } else {
                    vErrors.push(err90);
                  }
                  errors++;
                }
                if (data.workDelegationEdgeIds === void 0) {
                  const err91 = { instancePath, schemaPath: "#/oneOf/7/required", keyword: "required", params: { missingProperty: "workDelegationEdgeIds" }, message: "must have required property 'workDelegationEdgeIds'" };
                  if (vErrors === null) {
                    vErrors = [err91];
                  } else {
                    vErrors.push(err91);
                  }
                  errors++;
                }
                if (data.providerBindingIds === void 0) {
                  const err92 = { instancePath, schemaPath: "#/oneOf/7/required", keyword: "required", params: { missingProperty: "providerBindingIds" }, message: "must have required property 'providerBindingIds'" };
                  if (vErrors === null) {
                    vErrors = [err92];
                  } else {
                    vErrors.push(err92);
                  }
                  errors++;
                }
                if (data.capabilityCeiling === void 0) {
                  const err93 = { instancePath, schemaPath: "#/oneOf/7/required", keyword: "required", params: { missingProperty: "capabilityCeiling" }, message: "must have required property 'capabilityCeiling'" };
                  if (vErrors === null) {
                    vErrors = [err93];
                  } else {
                    vErrors.push(err93);
                  }
                  errors++;
                }
                if (data.effectCeiling === void 0) {
                  const err94 = { instancePath, schemaPath: "#/oneOf/7/required", keyword: "required", params: { missingProperty: "effectCeiling" }, message: "must have required property 'effectCeiling'" };
                  if (vErrors === null) {
                    vErrors = [err94];
                  } else {
                    vErrors.push(err94);
                  }
                  errors++;
                }
                if (data.terminalProviderBinding === void 0) {
                  const err95 = { instancePath, schemaPath: "#/oneOf/7/required", keyword: "required", params: { missingProperty: "terminalProviderBinding" }, message: "must have required property 'terminalProviderBinding'" };
                  if (vErrors === null) {
                    vErrors = [err95];
                  } else {
                    vErrors.push(err95);
                  }
                  errors++;
                }
                if (data.nativeRealization === void 0) {
                  const err96 = { instancePath, schemaPath: "#/oneOf/7/required", keyword: "required", params: { missingProperty: "nativeRealization" }, message: "must have required property 'nativeRealization'" };
                  if (vErrors === null) {
                    vErrors = [err96];
                  } else {
                    vErrors.push(err96);
                  }
                  errors++;
                }
                if (data.reasonSteps === void 0) {
                  const err97 = { instancePath, schemaPath: "#/oneOf/7/required", keyword: "required", params: { missingProperty: "reasonSteps" }, message: "must have required property 'reasonSteps'" };
                  if (vErrors === null) {
                    vErrors = [err97];
                  } else {
                    vErrors.push(err97);
                  }
                  errors++;
                }
                if (data.subject === void 0) {
                  const err98 = { instancePath, schemaPath: "#/oneOf/7/required", keyword: "required", params: { missingProperty: "subject" }, message: "must have required property 'subject'" };
                  if (vErrors === null) {
                    vErrors = [err98];
                  } else {
                    vErrors.push(err98);
                  }
                  errors++;
                }
                if (data.capabilityCeiling !== void 0) {
                  let data48 = data.capabilityCeiling;
                  if (Array.isArray(data48)) {
                    const len13 = data48.length;
                    for (let i13 = 0; i13 < len13; i13++) {
                      if (typeof data48[i13] !== "string") {
                        const err99 = { instancePath: instancePath + "/capabilityCeiling/" + i13, schemaPath: "#/oneOf/7/properties/capabilityCeiling/items/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                        if (vErrors === null) {
                          vErrors = [err99];
                        } else {
                          vErrors.push(err99);
                        }
                        errors++;
                      }
                    }
                  } else {
                    const err100 = { instancePath: instancePath + "/capabilityCeiling", schemaPath: "#/oneOf/7/properties/capabilityCeiling/type", keyword: "type", params: { type: "array" }, message: "must be array" };
                    if (vErrors === null) {
                      vErrors = [err100];
                    } else {
                      vErrors.push(err100);
                    }
                    errors++;
                  }
                }
                if (data.effectCeiling !== void 0) {
                  let data50 = data.effectCeiling;
                  if (Array.isArray(data50)) {
                    const len14 = data50.length;
                    for (let i14 = 0; i14 < len14; i14++) {
                      if (!validate196(data50[i14], { instancePath: instancePath + "/effectCeiling/" + i14, parentData: data50, parentDataProperty: i14, rootData, dynamicAnchors })) {
                        vErrors = vErrors === null ? validate196.errors : vErrors.concat(validate196.errors);
                        errors = vErrors.length;
                      }
                    }
                  } else {
                    const err101 = { instancePath: instancePath + "/effectCeiling", schemaPath: "#/oneOf/7/properties/effectCeiling/type", keyword: "type", params: { type: "array" }, message: "must be array" };
                    if (vErrors === null) {
                      vErrors = [err101];
                    } else {
                      vErrors.push(err101);
                    }
                    errors++;
                  }
                }
                if (data.experiencePathId !== void 0) {
                  if (typeof data.experiencePathId !== "string") {
                    const err102 = { instancePath: instancePath + "/experiencePathId", schemaPath: "#/oneOf/7/properties/experiencePathId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                    if (vErrors === null) {
                      vErrors = [err102];
                    } else {
                      vErrors.push(err102);
                    }
                    errors++;
                  }
                }
                if (data.kind !== void 0) {
                  let data53 = data.kind;
                  if (typeof data53 !== "string") {
                    const err103 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/7/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                    if (vErrors === null) {
                      vErrors = [err103];
                    } else {
                      vErrors.push(err103);
                    }
                    errors++;
                  }
                  if ("execution-path" !== data53) {
                    const err104 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/7/properties/kind/const", keyword: "const", params: { allowedValue: "execution-path" }, message: "must be equal to constant" };
                    if (vErrors === null) {
                      vErrors = [err104];
                    } else {
                      vErrors.push(err104);
                    }
                    errors++;
                  }
                }
                if (data.nativeRealization !== void 0) {
                  if (!validate165(data.nativeRealization, { instancePath: instancePath + "/nativeRealization", parentData: data, parentDataProperty: "nativeRealization", rootData, dynamicAnchors })) {
                    vErrors = vErrors === null ? validate165.errors : vErrors.concat(validate165.errors);
                    errors = vErrors.length;
                  }
                }
                if (data.providerBindingIds !== void 0) {
                  let data55 = data.providerBindingIds;
                  if (Array.isArray(data55)) {
                    const len15 = data55.length;
                    for (let i15 = 0; i15 < len15; i15++) {
                      if (typeof data55[i15] !== "string") {
                        const err105 = { instancePath: instancePath + "/providerBindingIds/" + i15, schemaPath: "#/oneOf/7/properties/providerBindingIds/items/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                        if (vErrors === null) {
                          vErrors = [err105];
                        } else {
                          vErrors.push(err105);
                        }
                        errors++;
                      }
                    }
                  } else {
                    const err106 = { instancePath: instancePath + "/providerBindingIds", schemaPath: "#/oneOf/7/properties/providerBindingIds/type", keyword: "type", params: { type: "array" }, message: "must be array" };
                    if (vErrors === null) {
                      vErrors = [err106];
                    } else {
                      vErrors.push(err106);
                    }
                    errors++;
                  }
                }
                if (data.reasonSteps !== void 0) {
                  let data57 = data.reasonSteps;
                  if (Array.isArray(data57)) {
                    const len16 = data57.length;
                    for (let i16 = 0; i16 < len16; i16++) {
                      if (!validate157(data57[i16], { instancePath: instancePath + "/reasonSteps/" + i16, parentData: data57, parentDataProperty: i16, rootData, dynamicAnchors })) {
                        vErrors = vErrors === null ? validate157.errors : vErrors.concat(validate157.errors);
                        errors = vErrors.length;
                      }
                    }
                  } else {
                    const err107 = { instancePath: instancePath + "/reasonSteps", schemaPath: "#/oneOf/7/properties/reasonSteps/type", keyword: "type", params: { type: "array" }, message: "must be array" };
                    if (vErrors === null) {
                      vErrors = [err107];
                    } else {
                      vErrors.push(err107);
                    }
                    errors++;
                  }
                }
                if (data.responsibilityContractIds !== void 0) {
                  let data59 = data.responsibilityContractIds;
                  if (Array.isArray(data59)) {
                    const len17 = data59.length;
                    for (let i17 = 0; i17 < len17; i17++) {
                      if (typeof data59[i17] !== "string") {
                        const err108 = { instancePath: instancePath + "/responsibilityContractIds/" + i17, schemaPath: "#/oneOf/7/properties/responsibilityContractIds/items/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                        if (vErrors === null) {
                          vErrors = [err108];
                        } else {
                          vErrors.push(err108);
                        }
                        errors++;
                      }
                    }
                  } else {
                    const err109 = { instancePath: instancePath + "/responsibilityContractIds", schemaPath: "#/oneOf/7/properties/responsibilityContractIds/type", keyword: "type", params: { type: "array" }, message: "must be array" };
                    if (vErrors === null) {
                      vErrors = [err109];
                    } else {
                      vErrors.push(err109);
                    }
                    errors++;
                  }
                }
                if (data.responsibilityPathIdentity !== void 0) {
                  if (typeof data.responsibilityPathIdentity !== "string") {
                    const err110 = { instancePath: instancePath + "/responsibilityPathIdentity", schemaPath: "#/oneOf/7/properties/responsibilityPathIdentity/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                    if (vErrors === null) {
                      vErrors = [err110];
                    } else {
                      vErrors.push(err110);
                    }
                    errors++;
                  }
                }
                if (data.selectionId !== void 0) {
                  if (typeof data.selectionId !== "string") {
                    const err111 = { instancePath: instancePath + "/selectionId", schemaPath: "#/oneOf/7/properties/selectionId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                    if (vErrors === null) {
                      vErrors = [err111];
                    } else {
                      vErrors.push(err111);
                    }
                    errors++;
                  }
                }
                if (data.subject !== void 0) {
                  if (!validate203(data.subject, { instancePath: instancePath + "/subject", parentData: data, parentDataProperty: "subject", rootData, dynamicAnchors })) {
                    vErrors = vErrors === null ? validate203.errors : vErrors.concat(validate203.errors);
                    errors = vErrors.length;
                  }
                }
                if (data.terminalProviderBinding !== void 0) {
                  let data64 = data.terminalProviderBinding;
                  const _errs130 = errors;
                  let valid45 = false;
                  const _errs131 = errors;
                  if (!validate217(data64, { instancePath: instancePath + "/terminalProviderBinding", parentData: data, parentDataProperty: "terminalProviderBinding", rootData, dynamicAnchors })) {
                    vErrors = vErrors === null ? validate217.errors : vErrors.concat(validate217.errors);
                    errors = vErrors.length;
                  }
                  var _valid1 = _errs131 === errors;
                  valid45 = valid45 || _valid1;
                  const _errs132 = errors;
                  if (data64 !== null) {
                    const err112 = { instancePath: instancePath + "/terminalProviderBinding", schemaPath: "#/oneOf/7/properties/terminalProviderBinding/anyOf/1/type", keyword: "type", params: { type: "null" }, message: "must be null" };
                    if (vErrors === null) {
                      vErrors = [err112];
                    } else {
                      vErrors.push(err112);
                    }
                    errors++;
                  }
                  var _valid1 = _errs132 === errors;
                  valid45 = valid45 || _valid1;
                  if (!valid45) {
                    const err113 = { instancePath: instancePath + "/terminalProviderBinding", schemaPath: "#/oneOf/7/properties/terminalProviderBinding/anyOf", keyword: "anyOf", params: {}, message: "must match a schema in anyOf" };
                    if (vErrors === null) {
                      vErrors = [err113];
                    } else {
                      vErrors.push(err113);
                    }
                    errors++;
                  } else {
                    errors = _errs130;
                    if (vErrors !== null) {
                      if (_errs130) {
                        vErrors.length = _errs130;
                      } else {
                        vErrors = null;
                      }
                    }
                  }
                }
                if (data.workDelegationEdgeIds !== void 0) {
                  let data65 = data.workDelegationEdgeIds;
                  if (Array.isArray(data65)) {
                    const len18 = data65.length;
                    for (let i18 = 0; i18 < len18; i18++) {
                      if (typeof data65[i18] !== "string") {
                        const err114 = { instancePath: instancePath + "/workDelegationEdgeIds/" + i18, schemaPath: "#/oneOf/7/properties/workDelegationEdgeIds/items/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                        if (vErrors === null) {
                          vErrors = [err114];
                        } else {
                          vErrors.push(err114);
                        }
                        errors++;
                      }
                    }
                  } else {
                    const err115 = { instancePath: instancePath + "/workDelegationEdgeIds", schemaPath: "#/oneOf/7/properties/workDelegationEdgeIds/type", keyword: "type", params: { type: "array" }, message: "must be array" };
                    if (vErrors === null) {
                      vErrors = [err115];
                    } else {
                      vErrors.push(err115);
                    }
                    errors++;
                  }
                }
              } else {
                const err116 = { instancePath, schemaPath: "#/oneOf/7/type", keyword: "type", params: { type: "object" }, message: "must be object" };
                if (vErrors === null) {
                  vErrors = [err116];
                } else {
                  vErrors.push(err116);
                }
                errors++;
              }
              var _valid0 = _errs99 === errors;
              if (_valid0 && valid0) {
                valid0 = false;
                passing0 = [passing0, 7];
              } else {
                if (_valid0) {
                  valid0 = true;
                  passing0 = 7;
                  if (props1 !== true) {
                    props1 = props1 || {};
                    props1.capabilityCeiling = true;
                    props1.effectCeiling = true;
                    props1.experiencePathId = true;
                    props1.kind = true;
                    props1.nativeRealization = true;
                    props1.providerBindingIds = true;
                    props1.reasonSteps = true;
                    props1.responsibilityContractIds = true;
                    props1.responsibilityPathIdentity = true;
                    props1.selectionId = true;
                    props1.subject = true;
                    props1.terminalProviderBinding = true;
                    props1.workDelegationEdgeIds = true;
                  }
                }
                const _errs138 = errors;
                if (data && typeof data == "object" && !Array.isArray(data)) {
                  if (data.kind === void 0) {
                    const err117 = { instancePath, schemaPath: "#/oneOf/8/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
                    if (vErrors === null) {
                      vErrors = [err117];
                    } else {
                      vErrors.push(err117);
                    }
                    errors++;
                  }
                  if (data.repositoryId === void 0) {
                    const err118 = { instancePath, schemaPath: "#/oneOf/8/required", keyword: "required", params: { missingProperty: "repositoryId" }, message: "must have required property 'repositoryId'" };
                    if (vErrors === null) {
                      vErrors = [err118];
                    } else {
                      vErrors.push(err118);
                    }
                    errors++;
                  }
                  if (data.operationId === void 0) {
                    const err119 = { instancePath, schemaPath: "#/oneOf/8/required", keyword: "required", params: { missingProperty: "operationId" }, message: "must have required property 'operationId'" };
                    if (vErrors === null) {
                      vErrors = [err119];
                    } else {
                      vErrors.push(err119);
                    }
                    errors++;
                  }
                  if (data.exposure === void 0) {
                    const err120 = { instancePath, schemaPath: "#/oneOf/8/required", keyword: "required", params: { missingProperty: "exposure" }, message: "must have required property 'exposure'" };
                    if (vErrors === null) {
                      vErrors = [err120];
                    } else {
                      vErrors.push(err120);
                    }
                    errors++;
                  }
                  if (data.nativeRealization === void 0) {
                    const err121 = { instancePath, schemaPath: "#/oneOf/8/required", keyword: "required", params: { missingProperty: "nativeRealization" }, message: "must have required property 'nativeRealization'" };
                    if (vErrors === null) {
                      vErrors = [err121];
                    } else {
                      vErrors.push(err121);
                    }
                    errors++;
                  }
                  if (data.exposure !== void 0) {
                    if (!validate163(data.exposure, { instancePath: instancePath + "/exposure", parentData: data, parentDataProperty: "exposure", rootData, dynamicAnchors })) {
                      vErrors = vErrors === null ? validate163.errors : vErrors.concat(validate163.errors);
                      errors = vErrors.length;
                    }
                  }
                  if (data.kind !== void 0) {
                    let data68 = data.kind;
                    if (typeof data68 !== "string") {
                      const err122 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/8/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                      if (vErrors === null) {
                        vErrors = [err122];
                      } else {
                        vErrors.push(err122);
                      }
                      errors++;
                    }
                    if ("operation" !== data68) {
                      const err123 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/8/properties/kind/const", keyword: "const", params: { allowedValue: "operation" }, message: "must be equal to constant" };
                      if (vErrors === null) {
                        vErrors = [err123];
                      } else {
                        vErrors.push(err123);
                      }
                      errors++;
                    }
                  }
                  if (data.nativeRealization !== void 0) {
                    if (!validate165(data.nativeRealization, { instancePath: instancePath + "/nativeRealization", parentData: data, parentDataProperty: "nativeRealization", rootData, dynamicAnchors })) {
                      vErrors = vErrors === null ? validate165.errors : vErrors.concat(validate165.errors);
                      errors = vErrors.length;
                    }
                  }
                  if (data.operationId !== void 0) {
                    if (typeof data.operationId !== "string") {
                      const err124 = { instancePath: instancePath + "/operationId", schemaPath: "#/oneOf/8/properties/operationId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                      if (vErrors === null) {
                        vErrors = [err124];
                      } else {
                        vErrors.push(err124);
                      }
                      errors++;
                    }
                  }
                  if (data.repositoryId !== void 0) {
                    if (typeof data.repositoryId !== "string") {
                      const err125 = { instancePath: instancePath + "/repositoryId", schemaPath: "#/oneOf/8/properties/repositoryId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                      if (vErrors === null) {
                        vErrors = [err125];
                      } else {
                        vErrors.push(err125);
                      }
                      errors++;
                    }
                  }
                } else {
                  const err126 = { instancePath, schemaPath: "#/oneOf/8/type", keyword: "type", params: { type: "object" }, message: "must be object" };
                  if (vErrors === null) {
                    vErrors = [err126];
                  } else {
                    vErrors.push(err126);
                  }
                  errors++;
                }
                var _valid0 = _errs138 === errors;
                if (_valid0 && valid0) {
                  valid0 = false;
                  passing0 = [passing0, 8];
                } else {
                  if (_valid0) {
                    valid0 = true;
                    passing0 = 8;
                    if (props1 !== true) {
                      props1 = props1 || {};
                      props1.exposure = true;
                      props1.kind = true;
                      props1.nativeRealization = true;
                      props1.operationId = true;
                      props1.repositoryId = true;
                    }
                  }
                  const _errs148 = errors;
                  if (data && typeof data == "object" && !Array.isArray(data)) {
                    if (data.kind === void 0) {
                      const err127 = { instancePath, schemaPath: "#/oneOf/9/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
                      if (vErrors === null) {
                        vErrors = [err127];
                      } else {
                        vErrors.push(err127);
                      }
                      errors++;
                    }
                    if (data.capabilityId === void 0) {
                      const err128 = { instancePath, schemaPath: "#/oneOf/9/required", keyword: "required", params: { missingProperty: "capabilityId" }, message: "must have required property 'capabilityId'" };
                      if (vErrors === null) {
                        vErrors = [err128];
                      } else {
                        vErrors.push(err128);
                      }
                      errors++;
                    }
                    if (data.repositoryId === void 0) {
                      const err129 = { instancePath, schemaPath: "#/oneOf/9/required", keyword: "required", params: { missingProperty: "repositoryId" }, message: "must have required property 'repositoryId'" };
                      if (vErrors === null) {
                        vErrors = [err129];
                      } else {
                        vErrors.push(err129);
                      }
                      errors++;
                    }
                    if (data.operationId === void 0) {
                      const err130 = { instancePath, schemaPath: "#/oneOf/9/required", keyword: "required", params: { missingProperty: "operationId" }, message: "must have required property 'operationId'" };
                      if (vErrors === null) {
                        vErrors = [err130];
                      } else {
                        vErrors.push(err130);
                      }
                      errors++;
                    }
                    if (data.requirementId === void 0) {
                      const err131 = { instancePath, schemaPath: "#/oneOf/9/required", keyword: "required", params: { missingProperty: "requirementId" }, message: "must have required property 'requirementId'" };
                      if (vErrors === null) {
                        vErrors = [err131];
                      } else {
                        vErrors.push(err131);
                      }
                      errors++;
                    }
                    if (data.effectClass === void 0) {
                      const err132 = { instancePath, schemaPath: "#/oneOf/9/required", keyword: "required", params: { missingProperty: "effectClass" }, message: "must have required property 'effectClass'" };
                      if (vErrors === null) {
                        vErrors = [err132];
                      } else {
                        vErrors.push(err132);
                      }
                      errors++;
                    }
                    if (data.requirementDisposition === void 0) {
                      const err133 = { instancePath, schemaPath: "#/oneOf/9/required", keyword: "required", params: { missingProperty: "requirementDisposition" }, message: "must have required property 'requirementDisposition'" };
                      if (vErrors === null) {
                        vErrors = [err133];
                      } else {
                        vErrors.push(err133);
                      }
                      errors++;
                    }
                    if (data.providerDisposition === void 0) {
                      const err134 = { instancePath, schemaPath: "#/oneOf/9/required", keyword: "required", params: { missingProperty: "providerDisposition" }, message: "must have required property 'providerDisposition'" };
                      if (vErrors === null) {
                        vErrors = [err134];
                      } else {
                        vErrors.push(err134);
                      }
                      errors++;
                    }
                    if (data.nativeRealization === void 0) {
                      const err135 = { instancePath, schemaPath: "#/oneOf/9/required", keyword: "required", params: { missingProperty: "nativeRealization" }, message: "must have required property 'nativeRealization'" };
                      if (vErrors === null) {
                        vErrors = [err135];
                      } else {
                        vErrors.push(err135);
                      }
                      errors++;
                    }
                    if (data.capabilityId !== void 0) {
                      if (typeof data.capabilityId !== "string") {
                        const err136 = { instancePath: instancePath + "/capabilityId", schemaPath: "#/oneOf/9/properties/capabilityId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                        if (vErrors === null) {
                          vErrors = [err136];
                        } else {
                          vErrors.push(err136);
                        }
                        errors++;
                      }
                    }
                    if (data.effectClass !== void 0) {
                      if (!validate196(data.effectClass, { instancePath: instancePath + "/effectClass", parentData: data, parentDataProperty: "effectClass", rootData, dynamicAnchors })) {
                        vErrors = vErrors === null ? validate196.errors : vErrors.concat(validate196.errors);
                        errors = vErrors.length;
                      }
                    }
                    if (data.kind !== void 0) {
                      let data74 = data.kind;
                      if (typeof data74 !== "string") {
                        const err137 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/9/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                        if (vErrors === null) {
                          vErrors = [err137];
                        } else {
                          vErrors.push(err137);
                        }
                        errors++;
                      }
                      if ("capability" !== data74) {
                        const err138 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/9/properties/kind/const", keyword: "const", params: { allowedValue: "capability" }, message: "must be equal to constant" };
                        if (vErrors === null) {
                          vErrors = [err138];
                        } else {
                          vErrors.push(err138);
                        }
                        errors++;
                      }
                    }
                    if (data.nativeRealization !== void 0) {
                      if (!validate165(data.nativeRealization, { instancePath: instancePath + "/nativeRealization", parentData: data, parentDataProperty: "nativeRealization", rootData, dynamicAnchors })) {
                        vErrors = vErrors === null ? validate165.errors : vErrors.concat(validate165.errors);
                        errors = vErrors.length;
                      }
                    }
                    if (data.operationId !== void 0) {
                      if (typeof data.operationId !== "string") {
                        const err139 = { instancePath: instancePath + "/operationId", schemaPath: "#/oneOf/9/properties/operationId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                        if (vErrors === null) {
                          vErrors = [err139];
                        } else {
                          vErrors.push(err139);
                        }
                        errors++;
                      }
                    }
                    if (data.providerDisposition !== void 0) {
                      if (!validate168(data.providerDisposition, { instancePath: instancePath + "/providerDisposition", parentData: data, parentDataProperty: "providerDisposition", rootData, dynamicAnchors })) {
                        vErrors = vErrors === null ? validate168.errors : vErrors.concat(validate168.errors);
                        errors = vErrors.length;
                      }
                    }
                    if (data.repositoryId !== void 0) {
                      if (typeof data.repositoryId !== "string") {
                        const err140 = { instancePath: instancePath + "/repositoryId", schemaPath: "#/oneOf/9/properties/repositoryId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                        if (vErrors === null) {
                          vErrors = [err140];
                        } else {
                          vErrors.push(err140);
                        }
                        errors++;
                      }
                    }
                    if (data.requirementDisposition !== void 0) {
                      if (!validate172(data.requirementDisposition, { instancePath: instancePath + "/requirementDisposition", parentData: data, parentDataProperty: "requirementDisposition", rootData, dynamicAnchors })) {
                        vErrors = vErrors === null ? validate172.errors : vErrors.concat(validate172.errors);
                        errors = vErrors.length;
                      }
                    }
                    if (data.requirementId !== void 0) {
                      if (typeof data.requirementId !== "string") {
                        const err141 = { instancePath: instancePath + "/requirementId", schemaPath: "#/oneOf/9/properties/requirementId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                        if (vErrors === null) {
                          vErrors = [err141];
                        } else {
                          vErrors.push(err141);
                        }
                        errors++;
                      }
                    }
                  } else {
                    const err142 = { instancePath, schemaPath: "#/oneOf/9/type", keyword: "type", params: { type: "object" }, message: "must be object" };
                    if (vErrors === null) {
                      vErrors = [err142];
                    } else {
                      vErrors.push(err142);
                    }
                    errors++;
                  }
                  var _valid0 = _errs148 === errors;
                  if (_valid0 && valid0) {
                    valid0 = false;
                    passing0 = [passing0, 9];
                  } else {
                    if (_valid0) {
                      valid0 = true;
                      passing0 = 9;
                      if (props1 !== true) {
                        props1 = props1 || {};
                        props1.capabilityId = true;
                        props1.effectClass = true;
                        props1.kind = true;
                        props1.nativeRealization = true;
                        props1.operationId = true;
                        props1.providerDisposition = true;
                        props1.repositoryId = true;
                        props1.requirementDisposition = true;
                        props1.requirementId = true;
                      }
                    }
                    const _errs164 = errors;
                    if (data && typeof data == "object" && !Array.isArray(data)) {
                      if (data.kind === void 0) {
                        const err143 = { instancePath, schemaPath: "#/oneOf/10/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
                        if (vErrors === null) {
                          vErrors = [err143];
                        } else {
                          vErrors.push(err143);
                        }
                        errors++;
                      }
                      if (data.machineId === void 0) {
                        const err144 = { instancePath, schemaPath: "#/oneOf/10/required", keyword: "required", params: { missingProperty: "machineId" }, message: "must have required property 'machineId'" };
                        if (vErrors === null) {
                          vErrors = [err144];
                        } else {
                          vErrors.push(err144);
                        }
                        errors++;
                      }
                      if (data.realization === void 0) {
                        const err145 = { instancePath, schemaPath: "#/oneOf/10/required", keyword: "required", params: { missingProperty: "realization" }, message: "must have required property 'realization'" };
                        if (vErrors === null) {
                          vErrors = [err145];
                        } else {
                          vErrors.push(err145);
                        }
                        errors++;
                      }
                      if (data.kind !== void 0) {
                        let data81 = data.kind;
                        if (typeof data81 !== "string") {
                          const err146 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/10/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                          if (vErrors === null) {
                            vErrors = [err146];
                          } else {
                            vErrors.push(err146);
                          }
                          errors++;
                        }
                        if ("machine-realization" !== data81) {
                          const err147 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/10/properties/kind/const", keyword: "const", params: { allowedValue: "machine-realization" }, message: "must be equal to constant" };
                          if (vErrors === null) {
                            vErrors = [err147];
                          } else {
                            vErrors.push(err147);
                          }
                          errors++;
                        }
                      }
                      if (data.machineId !== void 0) {
                        if (typeof data.machineId !== "string") {
                          const err148 = { instancePath: instancePath + "/machineId", schemaPath: "#/oneOf/10/properties/machineId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                          if (vErrors === null) {
                            vErrors = [err148];
                          } else {
                            vErrors.push(err148);
                          }
                          errors++;
                        }
                      }
                      if (data.realization !== void 0) {
                        if (!validate165(data.realization, { instancePath: instancePath + "/realization", parentData: data, parentDataProperty: "realization", rootData, dynamicAnchors })) {
                          vErrors = vErrors === null ? validate165.errors : vErrors.concat(validate165.errors);
                          errors = vErrors.length;
                        }
                      }
                    } else {
                      const err149 = { instancePath, schemaPath: "#/oneOf/10/type", keyword: "type", params: { type: "object" }, message: "must be object" };
                      if (vErrors === null) {
                        vErrors = [err149];
                      } else {
                        vErrors.push(err149);
                      }
                      errors++;
                    }
                    var _valid0 = _errs164 === errors;
                    if (_valid0 && valid0) {
                      valid0 = false;
                      passing0 = [passing0, 10];
                    } else {
                      if (_valid0) {
                        valid0 = true;
                        passing0 = 10;
                        if (props1 !== true) {
                          props1 = props1 || {};
                          props1.kind = true;
                          props1.machineId = true;
                          props1.realization = true;
                        }
                      }
                      const _errs171 = errors;
                      if (data && typeof data == "object" && !Array.isArray(data)) {
                        if (data.kind === void 0) {
                          const err150 = { instancePath, schemaPath: "#/oneOf/11/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
                          if (vErrors === null) {
                            vErrors = [err150];
                          } else {
                            vErrors.push(err150);
                          }
                          errors++;
                        }
                        if (data.finding === void 0) {
                          const err151 = { instancePath, schemaPath: "#/oneOf/11/required", keyword: "required", params: { missingProperty: "finding" }, message: "must have required property 'finding'" };
                          if (vErrors === null) {
                            vErrors = [err151];
                          } else {
                            vErrors.push(err151);
                          }
                          errors++;
                        }
                        if (data.finding !== void 0) {
                          if (!validate178(data.finding, { instancePath: instancePath + "/finding", parentData: data, parentDataProperty: "finding", rootData, dynamicAnchors })) {
                            vErrors = vErrors === null ? validate178.errors : vErrors.concat(validate178.errors);
                            errors = vErrors.length;
                          }
                        }
                        if (data.kind !== void 0) {
                          let data85 = data.kind;
                          if (typeof data85 !== "string") {
                            const err152 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/11/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
                            if (vErrors === null) {
                              vErrors = [err152];
                            } else {
                              vErrors.push(err152);
                            }
                            errors++;
                          }
                          if ("finding" !== data85) {
                            const err153 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/11/properties/kind/const", keyword: "const", params: { allowedValue: "finding" }, message: "must be equal to constant" };
                            if (vErrors === null) {
                              vErrors = [err153];
                            } else {
                              vErrors.push(err153);
                            }
                            errors++;
                          }
                        }
                      } else {
                        const err154 = { instancePath, schemaPath: "#/oneOf/11/type", keyword: "type", params: { type: "object" }, message: "must be object" };
                        if (vErrors === null) {
                          vErrors = [err154];
                        } else {
                          vErrors.push(err154);
                        }
                        errors++;
                      }
                      var _valid0 = _errs171 === errors;
                      if (_valid0 && valid0) {
                        valid0 = false;
                        passing0 = [passing0, 11];
                      } else {
                        if (_valid0) {
                          valid0 = true;
                          passing0 = 11;
                          if (props1 !== true) {
                            props1 = props1 || {};
                            props1.finding = true;
                            props1.kind = true;
                          }
                        }
                      }
                    }
                  }
                }
              }
            }
          }
        }
      }
    }
  }
  if (!valid0) {
    const err155 = { instancePath, schemaPath: "#/oneOf", keyword: "oneOf", params: { passingSchemas: passing0 }, message: "must match exactly one schema in oneOf" };
    if (vErrors === null) {
      vErrors = [err155];
    } else {
      vErrors.push(err155);
    }
    errors++;
  } else {
    errors = _errs1;
    if (vErrors !== null) {
      if (_errs1) {
        vErrors.length = _errs1;
      } else {
        vErrors = null;
      }
    }
  }
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.explanationHandle === void 0) {
      const err156 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "explanationHandle" }, message: "must have required property 'explanationHandle'" };
      if (vErrors === null) {
        vErrors = [err156];
      } else {
        vErrors.push(err156);
      }
      errors++;
    }
    if (props1 !== true) {
      props1 = props1 || {};
      props1.explanationHandle = true;
    }
    if (data.explanationHandle !== void 0) {
      if (!validate183(data.explanationHandle, { instancePath: instancePath + "/explanationHandle", parentData: data, parentDataProperty: "explanationHandle", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate183.errors : vErrors.concat(validate183.errors);
        errors = vErrors.length;
      }
    }
  } else {
    const err157 = { instancePath, schemaPath: "#/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err157];
    } else {
      vErrors.push(err157);
    }
    errors++;
  }
  validate194.errors = vErrors;
  evaluated0.props = props1;
  return errors === 0;
}
validate194.evaluated = { "dynamicProps": true, "dynamicItems": false };
function validate229(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate229.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  const _errs0 = errors;
  let valid0 = false;
  let passing0 = null;
  const _errs1 = errors;
  if (typeof data !== "string") {
    const err0 = { instancePath, schemaPath: "#/oneOf/0/type", keyword: "type", params: { type: "string" }, message: "must be string" };
    if (vErrors === null) {
      vErrors = [err0];
    } else {
      vErrors.push(err0);
    }
    errors++;
  }
  if ("intended-use-case" !== data) {
    const err1 = { instancePath, schemaPath: "#/oneOf/0/const", keyword: "const", params: { allowedValue: "intended-use-case" }, message: "must be equal to constant" };
    if (vErrors === null) {
      vErrors = [err1];
    } else {
      vErrors.push(err1);
    }
    errors++;
  }
  var _valid0 = _errs1 === errors;
  if (_valid0) {
    valid0 = true;
    passing0 = 0;
  }
  const _errs3 = errors;
  if (typeof data !== "string") {
    const err2 = { instancePath, schemaPath: "#/oneOf/1/type", keyword: "type", params: { type: "string" }, message: "must be string" };
    if (vErrors === null) {
      vErrors = [err2];
    } else {
      vErrors.push(err2);
    }
    errors++;
  }
  if ("effective-configuration" !== data) {
    const err3 = { instancePath, schemaPath: "#/oneOf/1/const", keyword: "const", params: { allowedValue: "effective-configuration" }, message: "must be equal to constant" };
    if (vErrors === null) {
      vErrors = [err3];
    } else {
      vErrors.push(err3);
    }
    errors++;
  }
  var _valid0 = _errs3 === errors;
  if (_valid0 && valid0) {
    valid0 = false;
    passing0 = [passing0, 1];
  } else {
    if (_valid0) {
      valid0 = true;
      passing0 = 1;
    }
    const _errs5 = errors;
    if (typeof data !== "string") {
      const err4 = { instancePath, schemaPath: "#/oneOf/2/type", keyword: "type", params: { type: "string" }, message: "must be string" };
      if (vErrors === null) {
        vErrors = [err4];
      } else {
        vErrors.push(err4);
      }
      errors++;
    }
    if ("capabilities-and-operations" !== data) {
      const err5 = { instancePath, schemaPath: "#/oneOf/2/const", keyword: "const", params: { allowedValue: "capabilities-and-operations" }, message: "must be equal to constant" };
      if (vErrors === null) {
        vErrors = [err5];
      } else {
        vErrors.push(err5);
      }
      errors++;
    }
    var _valid0 = _errs5 === errors;
    if (_valid0 && valid0) {
      valid0 = false;
      passing0 = [passing0, 2];
    } else {
      if (_valid0) {
        valid0 = true;
        passing0 = 2;
      }
      const _errs7 = errors;
      if (typeof data !== "string") {
        const err6 = { instancePath, schemaPath: "#/oneOf/3/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err6];
        } else {
          vErrors.push(err6);
        }
        errors++;
      }
      if ("machine-realization" !== data) {
        const err7 = { instancePath, schemaPath: "#/oneOf/3/const", keyword: "const", params: { allowedValue: "machine-realization" }, message: "must be equal to constant" };
        if (vErrors === null) {
          vErrors = [err7];
        } else {
          vErrors.push(err7);
        }
        errors++;
      }
      var _valid0 = _errs7 === errors;
      if (_valid0 && valid0) {
        valid0 = false;
        passing0 = [passing0, 3];
      } else {
        if (_valid0) {
          valid0 = true;
          passing0 = 3;
        }
        const _errs9 = errors;
        if (typeof data !== "string") {
          const err8 = { instancePath, schemaPath: "#/oneOf/4/type", keyword: "type", params: { type: "string" }, message: "must be string" };
          if (vErrors === null) {
            vErrors = [err8];
          } else {
            vErrors.push(err8);
          }
          errors++;
        }
        if ("gaps-and-explanations" !== data) {
          const err9 = { instancePath, schemaPath: "#/oneOf/4/const", keyword: "const", params: { allowedValue: "gaps-and-explanations" }, message: "must be equal to constant" };
          if (vErrors === null) {
            vErrors = [err9];
          } else {
            vErrors.push(err9);
          }
          errors++;
        }
        var _valid0 = _errs9 === errors;
        if (_valid0 && valid0) {
          valid0 = false;
          passing0 = [passing0, 4];
        } else {
          if (_valid0) {
            valid0 = true;
            passing0 = 4;
          }
        }
      }
    }
  }
  if (!valid0) {
    const err10 = { instancePath, schemaPath: "#/oneOf", keyword: "oneOf", params: { passingSchemas: passing0 }, message: "must match exactly one schema in oneOf" };
    if (vErrors === null) {
      vErrors = [err10];
    } else {
      vErrors.push(err10);
    }
    errors++;
  } else {
    errors = _errs0;
    if (vErrors !== null) {
      if (_errs0) {
        vErrors.length = _errs0;
      } else {
        vErrors = null;
      }
    }
  }
  validate229.errors = vErrors;
  return errors === 0;
}
validate229.evaluated = { "dynamicProps": false, "dynamicItems": false };
function validate193(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate193.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.section === void 0) {
      const err0 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "section" }, message: "must have required property 'section'" };
      if (vErrors === null) {
        vErrors = [err0];
      } else {
        vErrors.push(err0);
      }
      errors++;
    }
    if (data.offset === void 0) {
      const err1 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "offset" }, message: "must have required property 'offset'" };
      if (vErrors === null) {
        vErrors = [err1];
      } else {
        vErrors.push(err1);
      }
      errors++;
    }
    if (data.entries === void 0) {
      const err2 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "entries" }, message: "must have required property 'entries'" };
      if (vErrors === null) {
        vErrors = [err2];
      } else {
        vErrors.push(err2);
      }
      errors++;
    }
    if (data.nextOffset === void 0) {
      const err3 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "nextOffset" }, message: "must have required property 'nextOffset'" };
      if (vErrors === null) {
        vErrors = [err3];
      } else {
        vErrors.push(err3);
      }
      errors++;
    }
    for (const key0 in data) {
      if (!(key0 === "entries" || key0 === "nextOffset" || key0 === "offset" || key0 === "section")) {
        const err4 = { instancePath, schemaPath: "#/additionalProperties", keyword: "additionalProperties", params: { additionalProperty: key0 }, message: "must NOT have additional properties" };
        if (vErrors === null) {
          vErrors = [err4];
        } else {
          vErrors.push(err4);
        }
        errors++;
      }
    }
    if (data.entries !== void 0) {
      let data0 = data.entries;
      if (Array.isArray(data0)) {
        const len0 = data0.length;
        for (let i0 = 0; i0 < len0; i0++) {
          if (!validate194(data0[i0], { instancePath: instancePath + "/entries/" + i0, parentData: data0, parentDataProperty: i0, rootData, dynamicAnchors })) {
            vErrors = vErrors === null ? validate194.errors : vErrors.concat(validate194.errors);
            errors = vErrors.length;
          }
        }
      } else {
        const err5 = { instancePath: instancePath + "/entries", schemaPath: "#/properties/entries/type", keyword: "type", params: { type: "array" }, message: "must be array" };
        if (vErrors === null) {
          vErrors = [err5];
        } else {
          vErrors.push(err5);
        }
        errors++;
      }
    }
    if (data.nextOffset !== void 0) {
      let data2 = data.nextOffset;
      if (!(typeof data2 == "number" && (!(data2 % 1) && !isNaN(data2))) && data2 !== null) {
        const err6 = { instancePath: instancePath + "/nextOffset", schemaPath: "#/properties/nextOffset/type", keyword: "type", params: { type: schema73.properties.nextOffset.type }, message: "must be integer,null" };
        if (vErrors === null) {
          vErrors = [err6];
        } else {
          vErrors.push(err6);
        }
        errors++;
      }
      if (typeof data2 == "number") {
        if (data2 < 0 || isNaN(data2)) {
          const err7 = { instancePath: instancePath + "/nextOffset", schemaPath: "#/properties/nextOffset/minimum", keyword: "minimum", params: { comparison: ">=", limit: 0 }, message: "must be >= 0" };
          if (vErrors === null) {
            vErrors = [err7];
          } else {
            vErrors.push(err7);
          }
          errors++;
        }
        if (!formats4.validate(data2)) {
          const err8 = { instancePath: instancePath + "/nextOffset", schemaPath: "#/properties/nextOffset/format", keyword: "format", params: { format: "uint32" }, message: 'must match format "uint32"' };
          if (vErrors === null) {
            vErrors = [err8];
          } else {
            vErrors.push(err8);
          }
          errors++;
        }
      }
    }
    if (data.offset !== void 0) {
      let data3 = data.offset;
      if (!(typeof data3 == "number" && (!(data3 % 1) && !isNaN(data3)))) {
        const err9 = { instancePath: instancePath + "/offset", schemaPath: "#/properties/offset/type", keyword: "type", params: { type: "integer" }, message: "must be integer" };
        if (vErrors === null) {
          vErrors = [err9];
        } else {
          vErrors.push(err9);
        }
        errors++;
      }
      if (typeof data3 == "number") {
        if (data3 < 0 || isNaN(data3)) {
          const err10 = { instancePath: instancePath + "/offset", schemaPath: "#/properties/offset/minimum", keyword: "minimum", params: { comparison: ">=", limit: 0 }, message: "must be >= 0" };
          if (vErrors === null) {
            vErrors = [err10];
          } else {
            vErrors.push(err10);
          }
          errors++;
        }
        if (!formats4.validate(data3)) {
          const err11 = { instancePath: instancePath + "/offset", schemaPath: "#/properties/offset/format", keyword: "format", params: { format: "uint32" }, message: 'must match format "uint32"' };
          if (vErrors === null) {
            vErrors = [err11];
          } else {
            vErrors.push(err11);
          }
          errors++;
        }
      }
    }
    if (data.section !== void 0) {
      if (!validate229(data.section, { instancePath: instancePath + "/section", parentData: data, parentDataProperty: "section", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate229.errors : vErrors.concat(validate229.errors);
        errors = vErrors.length;
      }
    }
  } else {
    const err12 = { instancePath, schemaPath: "#/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err12];
    } else {
      vErrors.push(err12);
    }
    errors++;
  }
  validate193.errors = vErrors;
  return errors === 0;
}
validate193.evaluated = { "props": true, "dynamicProps": false, "dynamicItems": false };
function validate232(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate232.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  const _errs0 = errors;
  let valid0 = false;
  let passing0 = null;
  const _errs1 = errors;
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.kind === void 0) {
      const err0 = { instancePath, schemaPath: "#/oneOf/0/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
      if (vErrors === null) {
        vErrors = [err0];
      } else {
        vErrors.push(err0);
      }
      errors++;
    }
    if (data.kind !== void 0) {
      let data0 = data.kind;
      if (typeof data0 !== "string") {
        const err1 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/0/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err1];
        } else {
          vErrors.push(err1);
        }
        errors++;
      }
      if ("current" !== data0) {
        const err2 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/0/properties/kind/const", keyword: "const", params: { allowedValue: "current" }, message: "must be equal to constant" };
        if (vErrors === null) {
          vErrors = [err2];
        } else {
          vErrors.push(err2);
        }
        errors++;
      }
    }
  } else {
    const err3 = { instancePath, schemaPath: "#/oneOf/0/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err3];
    } else {
      vErrors.push(err3);
    }
    errors++;
  }
  var _valid0 = _errs1 === errors;
  if (_valid0) {
    valid0 = true;
    passing0 = 0;
    var props0 = {};
    props0.kind = true;
  }
  const _errs5 = errors;
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.kind === void 0) {
      const err4 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
      if (vErrors === null) {
        vErrors = [err4];
      } else {
        vErrors.push(err4);
      }
      errors++;
    }
    if (data.admittedEditor === void 0) {
      const err5 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "admittedEditor" }, message: "must have required property 'admittedEditor'" };
      if (vErrors === null) {
        vErrors = [err5];
      } else {
        vErrors.push(err5);
      }
      errors++;
    }
    if (data.admittedProfileDigest === void 0) {
      const err6 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "admittedProfileDigest" }, message: "must have required property 'admittedProfileDigest'" };
      if (vErrors === null) {
        vErrors = [err6];
      } else {
        vErrors.push(err6);
      }
      errors++;
    }
    if (data.summary === void 0) {
      const err7 = { instancePath, schemaPath: "#/oneOf/1/required", keyword: "required", params: { missingProperty: "summary" }, message: "must have required property 'summary'" };
      if (vErrors === null) {
        vErrors = [err7];
      } else {
        vErrors.push(err7);
      }
      errors++;
    }
    if (data.admittedEditor !== void 0) {
      if (!validate56(data.admittedEditor, { instancePath: instancePath + "/admittedEditor", parentData: data, parentDataProperty: "admittedEditor", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate56.errors : vErrors.concat(validate56.errors);
        errors = vErrors.length;
      }
    }
    if (data.admittedProfileDigest !== void 0) {
      if (typeof data.admittedProfileDigest !== "string") {
        const err8 = { instancePath: instancePath + "/admittedProfileDigest", schemaPath: "#/oneOf/1/properties/admittedProfileDigest/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err8];
        } else {
          vErrors.push(err8);
        }
        errors++;
      }
    }
    if (data.kind !== void 0) {
      let data3 = data.kind;
      if (typeof data3 !== "string") {
        const err9 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/1/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err9];
        } else {
          vErrors.push(err9);
        }
        errors++;
      }
      if ("stale" !== data3) {
        const err10 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/1/properties/kind/const", keyword: "const", params: { allowedValue: "stale" }, message: "must be equal to constant" };
        if (vErrors === null) {
          vErrors = [err10];
        } else {
          vErrors.push(err10);
        }
        errors++;
      }
    }
    if (data.summary !== void 0) {
      if (typeof data.summary !== "string") {
        const err11 = { instancePath: instancePath + "/summary", schemaPath: "#/oneOf/1/properties/summary/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err11];
        } else {
          vErrors.push(err11);
        }
        errors++;
      }
    }
  } else {
    const err12 = { instancePath, schemaPath: "#/oneOf/1/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err12];
    } else {
      vErrors.push(err12);
    }
    errors++;
  }
  var _valid0 = _errs5 === errors;
  if (_valid0 && valid0) {
    valid0 = false;
    passing0 = [passing0, 1];
  } else {
    if (_valid0) {
      valid0 = true;
      passing0 = 1;
      if (props0 !== true) {
        props0 = props0 || {};
        props0.admittedEditor = true;
        props0.admittedProfileDigest = true;
        props0.kind = true;
        props0.summary = true;
      }
    }
    const _errs14 = errors;
    if (data && typeof data == "object" && !Array.isArray(data)) {
      if (data.kind === void 0) {
        const err13 = { instancePath, schemaPath: "#/oneOf/2/required", keyword: "required", params: { missingProperty: "kind" }, message: "must have required property 'kind'" };
        if (vErrors === null) {
          vErrors = [err13];
        } else {
          vErrors.push(err13);
        }
        errors++;
      }
      if (data.summary === void 0) {
        const err14 = { instancePath, schemaPath: "#/oneOf/2/required", keyword: "required", params: { missingProperty: "summary" }, message: "must have required property 'summary'" };
        if (vErrors === null) {
          vErrors = [err14];
        } else {
          vErrors.push(err14);
        }
        errors++;
      }
      if (data.kind !== void 0) {
        let data5 = data.kind;
        if (typeof data5 !== "string") {
          const err15 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/2/properties/kind/type", keyword: "type", params: { type: "string" }, message: "must be string" };
          if (vErrors === null) {
            vErrors = [err15];
          } else {
            vErrors.push(err15);
          }
          errors++;
        }
        if ("not-admitted" !== data5) {
          const err16 = { instancePath: instancePath + "/kind", schemaPath: "#/oneOf/2/properties/kind/const", keyword: "const", params: { allowedValue: "not-admitted" }, message: "must be equal to constant" };
          if (vErrors === null) {
            vErrors = [err16];
          } else {
            vErrors.push(err16);
          }
          errors++;
        }
      }
      if (data.summary !== void 0) {
        if (typeof data.summary !== "string") {
          const err17 = { instancePath: instancePath + "/summary", schemaPath: "#/oneOf/2/properties/summary/type", keyword: "type", params: { type: "string" }, message: "must be string" };
          if (vErrors === null) {
            vErrors = [err17];
          } else {
            vErrors.push(err17);
          }
          errors++;
        }
      }
    } else {
      const err18 = { instancePath, schemaPath: "#/oneOf/2/type", keyword: "type", params: { type: "object" }, message: "must be object" };
      if (vErrors === null) {
        vErrors = [err18];
      } else {
        vErrors.push(err18);
      }
      errors++;
    }
    var _valid0 = _errs14 === errors;
    if (_valid0 && valid0) {
      valid0 = false;
      passing0 = [passing0, 2];
    } else {
      if (_valid0) {
        valid0 = true;
        passing0 = 2;
        if (props0 !== true) {
          props0 = props0 || {};
          props0.kind = true;
          props0.summary = true;
        }
      }
    }
  }
  if (!valid0) {
    const err19 = { instancePath, schemaPath: "#/oneOf", keyword: "oneOf", params: { passingSchemas: passing0 }, message: "must match exactly one schema in oneOf" };
    if (vErrors === null) {
      vErrors = [err19];
    } else {
      vErrors.push(err19);
    }
    errors++;
  } else {
    errors = _errs0;
    if (vErrors !== null) {
      if (_errs0) {
        vErrors.length = _errs0;
      } else {
        vErrors = null;
      }
    }
  }
  validate232.errors = vErrors;
  evaluated0.props = props0;
  return errors === 0;
}
validate232.evaluated = { "dynamicProps": true, "dynamicItems": false };
function validate273(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate273.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.state === void 0) {
      const err0 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "state" }, message: "must have required property 'state'" };
      if (vErrors === null) {
        vErrors = [err0];
      } else {
        vErrors.push(err0);
      }
      errors++;
    }
    if (data.editor === void 0) {
      const err1 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "editor" }, message: "must have required property 'editor'" };
      if (vErrors === null) {
        vErrors = [err1];
      } else {
        vErrors.push(err1);
      }
      errors++;
    }
    if (data.context === void 0) {
      const err2 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "context" }, message: "must have required property 'context'" };
      if (vErrors === null) {
        vErrors = [err2];
      } else {
        vErrors.push(err2);
      }
      errors++;
    }
    for (const key0 in data) {
      if (!(key0 === "context" || key0 === "coordinates" || key0 === "diagnosticIdentity" || key0 === "editor" || key0 === "page" || key0 === "state")) {
        const err3 = { instancePath, schemaPath: "#/additionalProperties", keyword: "additionalProperties", params: { additionalProperty: key0 }, message: "must NOT have additional properties" };
        if (vErrors === null) {
          vErrors = [err3];
        } else {
          vErrors.push(err3);
        }
        errors++;
      }
    }
    if (data.context !== void 0) {
      if (!validate79(data.context, { instancePath: instancePath + "/context", parentData: data, parentDataProperty: "context", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate79.errors : vErrors.concat(validate79.errors);
        errors = vErrors.length;
      }
    }
    if (data.coordinates !== void 0) {
      if (!validate144(data.coordinates, { instancePath: instancePath + "/coordinates", parentData: data, parentDataProperty: "coordinates", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate144.errors : vErrors.concat(validate144.errors);
        errors = vErrors.length;
      }
    }
    if (data.diagnosticIdentity !== void 0) {
      if (!validate190(data.diagnosticIdentity, { instancePath: instancePath + "/diagnosticIdentity", parentData: data, parentDataProperty: "diagnosticIdentity", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate190.errors : vErrors.concat(validate190.errors);
        errors = vErrors.length;
      }
    }
    if (data.editor !== void 0) {
      if (!validate56(data.editor, { instancePath: instancePath + "/editor", parentData: data, parentDataProperty: "editor", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate56.errors : vErrors.concat(validate56.errors);
        errors = vErrors.length;
      }
    }
    if (data.page !== void 0) {
      if (!validate193(data.page, { instancePath: instancePath + "/page", parentData: data, parentDataProperty: "page", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate193.errors : vErrors.concat(validate193.errors);
        errors = vErrors.length;
      }
    }
    if (data.state !== void 0) {
      if (!validate232(data.state, { instancePath: instancePath + "/state", parentData: data, parentDataProperty: "state", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate232.errors : vErrors.concat(validate232.errors);
        errors = vErrors.length;
      }
    }
  } else {
    const err4 = { instancePath, schemaPath: "#/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err4];
    } else {
      vErrors.push(err4);
    }
    errors++;
  }
  validate273.errors = vErrors;
  return errors === 0;
}
validate273.evaluated = { "props": true, "dynamicProps": false, "dynamicItems": false };
var validatePaleWorkspaceCapabilities = validate280;
function validate239(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate239.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  const _errs0 = errors;
  let valid0 = false;
  let passing0 = null;
  const _errs1 = errors;
  if (typeof data !== "string") {
    const err0 = { instancePath, schemaPath: "#/oneOf/0/type", keyword: "type", params: { type: "string" }, message: "must be string" };
    if (vErrors === null) {
      vErrors = [err0];
    } else {
      vErrors.push(err0);
    }
    errors++;
  }
  if ("result-suppression" !== data) {
    const err1 = { instancePath, schemaPath: "#/oneOf/0/const", keyword: "const", params: { allowedValue: "result-suppression" }, message: "must be equal to constant" };
    if (vErrors === null) {
      vErrors = [err1];
    } else {
      vErrors.push(err1);
    }
    errors++;
  }
  var _valid0 = _errs1 === errors;
  if (_valid0) {
    valid0 = true;
    passing0 = 0;
  }
  if (!valid0) {
    const err2 = { instancePath, schemaPath: "#/oneOf", keyword: "oneOf", params: { passingSchemas: passing0 }, message: "must match exactly one schema in oneOf" };
    if (vErrors === null) {
      vErrors = [err2];
    } else {
      vErrors.push(err2);
    }
    errors++;
  } else {
    errors = _errs0;
    if (vErrors !== null) {
      if (_errs0) {
        vErrors.length = _errs0;
      } else {
        vErrors = null;
      }
    }
  }
  validate239.errors = vErrors;
  return errors === 0;
}
validate239.evaluated = { "dynamicProps": false, "dynamicItems": false };
function validate280(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate280.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.packageVersion === void 0) {
      const err0 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "packageVersion" }, message: "must have required property 'packageVersion'" };
      if (vErrors === null) {
        vErrors = [err0];
      } else {
        vErrors.push(err0);
      }
      errors++;
    }
    if (data.inventorySha256 === void 0) {
      const err1 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "inventorySha256" }, message: "must have required property 'inventorySha256'" };
      if (vErrors === null) {
        vErrors = [err1];
      } else {
        vErrors.push(err1);
      }
      errors++;
    }
    if (data.wireFixtureSha256 === void 0) {
      const err2 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "wireFixtureSha256" }, message: "must have required property 'wireFixtureSha256'" };
      if (vErrors === null) {
        vErrors = [err2];
      } else {
        vErrors.push(err2);
      }
      errors++;
    }
    if (data.protocolId === void 0) {
      const err3 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "protocolId" }, message: "must have required property 'protocolId'" };
      if (vErrors === null) {
        vErrors = [err3];
      } else {
        vErrors.push(err3);
      }
      errors++;
    }
    if (data.methods === void 0) {
      const err4 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "methods" }, message: "must have required property 'methods'" };
      if (vErrors === null) {
        vErrors = [err4];
      } else {
        vErrors.push(err4);
      }
      errors++;
    }
    if (data.notifications === void 0) {
      const err5 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "notifications" }, message: "must have required property 'notifications'" };
      if (vErrors === null) {
        vErrors = [err5];
      } else {
        vErrors.push(err5);
      }
      errors++;
    }
    if (data.cancellation === void 0) {
      const err6 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "cancellation" }, message: "must have required property 'cancellation'" };
      if (vErrors === null) {
        vErrors = [err6];
      } else {
        vErrors.push(err6);
      }
      errors++;
    }
    if (data.documentRoles === void 0) {
      const err7 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "documentRoles" }, message: "must have required property 'documentRoles'" };
      if (vErrors === null) {
        vErrors = [err7];
      } else {
        vErrors.push(err7);
      }
      errors++;
    }
    for (const key0 in data) {
      if (!(key0 === "cancellation" || key0 === "documentRoles" || key0 === "inventorySha256" || key0 === "methods" || key0 === "notifications" || key0 === "packageVersion" || key0 === "protocolId" || key0 === "wireFixtureSha256")) {
        const err8 = { instancePath, schemaPath: "#/additionalProperties", keyword: "additionalProperties", params: { additionalProperty: key0 }, message: "must NOT have additional properties" };
        if (vErrors === null) {
          vErrors = [err8];
        } else {
          vErrors.push(err8);
        }
        errors++;
      }
    }
    if (data.cancellation !== void 0) {
      if (!validate239(data.cancellation, { instancePath: instancePath + "/cancellation", parentData: data, parentDataProperty: "cancellation", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate239.errors : vErrors.concat(validate239.errors);
        errors = vErrors.length;
      }
    }
    if (data.documentRoles !== void 0) {
      let data1 = data.documentRoles;
      if (Array.isArray(data1)) {
        const len0 = data1.length;
        for (let i0 = 0; i0 < len0; i0++) {
          if (typeof data1[i0] !== "string") {
            const err9 = { instancePath: instancePath + "/documentRoles/" + i0, schemaPath: "#/properties/documentRoles/items/type", keyword: "type", params: { type: "string" }, message: "must be string" };
            if (vErrors === null) {
              vErrors = [err9];
            } else {
              vErrors.push(err9);
            }
            errors++;
          }
        }
      } else {
        const err10 = { instancePath: instancePath + "/documentRoles", schemaPath: "#/properties/documentRoles/type", keyword: "type", params: { type: "array" }, message: "must be array" };
        if (vErrors === null) {
          vErrors = [err10];
        } else {
          vErrors.push(err10);
        }
        errors++;
      }
    }
    if (data.inventorySha256 !== void 0) {
      if (typeof data.inventorySha256 !== "string") {
        const err11 = { instancePath: instancePath + "/inventorySha256", schemaPath: "#/properties/inventorySha256/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err11];
        } else {
          vErrors.push(err11);
        }
        errors++;
      }
    }
    if (data.methods !== void 0) {
      let data4 = data.methods;
      if (Array.isArray(data4)) {
        const len1 = data4.length;
        for (let i1 = 0; i1 < len1; i1++) {
          if (typeof data4[i1] !== "string") {
            const err12 = { instancePath: instancePath + "/methods/" + i1, schemaPath: "#/properties/methods/items/type", keyword: "type", params: { type: "string" }, message: "must be string" };
            if (vErrors === null) {
              vErrors = [err12];
            } else {
              vErrors.push(err12);
            }
            errors++;
          }
        }
      } else {
        const err13 = { instancePath: instancePath + "/methods", schemaPath: "#/properties/methods/type", keyword: "type", params: { type: "array" }, message: "must be array" };
        if (vErrors === null) {
          vErrors = [err13];
        } else {
          vErrors.push(err13);
        }
        errors++;
      }
    }
    if (data.notifications !== void 0) {
      let data6 = data.notifications;
      if (Array.isArray(data6)) {
        if (data6.length > 1) {
          const err14 = { instancePath: instancePath + "/notifications", schemaPath: "#/properties/notifications/maxItems", keyword: "maxItems", params: { limit: 1 }, message: "must NOT have more than 1 items" };
          if (vErrors === null) {
            vErrors = [err14];
          } else {
            vErrors.push(err14);
          }
          errors++;
        }
        if (data6.length < 1) {
          const err15 = { instancePath: instancePath + "/notifications", schemaPath: "#/properties/notifications/minItems", keyword: "minItems", params: { limit: 1 }, message: "must NOT have fewer than 1 items" };
          if (vErrors === null) {
            vErrors = [err15];
          } else {
            vErrors.push(err15);
          }
          errors++;
        }
        const len2 = data6.length;
        for (let i2 = 0; i2 < len2; i2++) {
          if (typeof data6[i2] !== "string") {
            const err16 = { instancePath: instancePath + "/notifications/" + i2, schemaPath: "#/properties/notifications/items/type", keyword: "type", params: { type: "string" }, message: "must be string" };
            if (vErrors === null) {
              vErrors = [err16];
            } else {
              vErrors.push(err16);
            }
            errors++;
          }
        }
      } else {
        const err17 = { instancePath: instancePath + "/notifications", schemaPath: "#/properties/notifications/type", keyword: "type", params: { type: "array" }, message: "must be array" };
        if (vErrors === null) {
          vErrors = [err17];
        } else {
          vErrors.push(err17);
        }
        errors++;
      }
    }
    if (data.packageVersion !== void 0) {
      if (typeof data.packageVersion !== "string") {
        const err18 = { instancePath: instancePath + "/packageVersion", schemaPath: "#/properties/packageVersion/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err18];
        } else {
          vErrors.push(err18);
        }
        errors++;
      }
    }
    if (data.protocolId !== void 0) {
      if (typeof data.protocolId !== "string") {
        const err19 = { instancePath: instancePath + "/protocolId", schemaPath: "#/properties/protocolId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err19];
        } else {
          vErrors.push(err19);
        }
        errors++;
      }
    }
    if (data.wireFixtureSha256 !== void 0) {
      if (typeof data.wireFixtureSha256 !== "string") {
        const err20 = { instancePath: instancePath + "/wireFixtureSha256", schemaPath: "#/properties/wireFixtureSha256/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err20];
        } else {
          vErrors.push(err20);
        }
        errors++;
      }
    }
  } else {
    const err21 = { instancePath, schemaPath: "#/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err21];
    } else {
      vErrors.push(err21);
    }
    errors++;
  }
  validate280.errors = vErrors;
  return errors === 0;
}
validate280.evaluated = { "props": true, "dynamicProps": false, "dynamicItems": false };
var validateProfileContextsResponse = validate282;
function validate243(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate243.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.projectId === void 0) {
      const err0 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "projectId" }, message: "must have required property 'projectId'" };
      if (vErrors === null) {
        vErrors = [err0];
      } else {
        vErrors.push(err0);
      }
      errors++;
    }
    if (data.executionWorkspaceId === void 0) {
      const err1 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "executionWorkspaceId" }, message: "must have required property 'executionWorkspaceId'" };
      if (vErrors === null) {
        vErrors = [err1];
      } else {
        vErrors.push(err1);
      }
      errors++;
    }
    for (const key0 in data) {
      if (!(key0 === "executionWorkspaceId" || key0 === "projectId")) {
        const err2 = { instancePath, schemaPath: "#/additionalProperties", keyword: "additionalProperties", params: { additionalProperty: key0 }, message: "must NOT have additional properties" };
        if (vErrors === null) {
          vErrors = [err2];
        } else {
          vErrors.push(err2);
        }
        errors++;
      }
    }
    if (data.executionWorkspaceId !== void 0) {
      if (typeof data.executionWorkspaceId !== "string") {
        const err3 = { instancePath: instancePath + "/executionWorkspaceId", schemaPath: "#/properties/executionWorkspaceId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err3];
        } else {
          vErrors.push(err3);
        }
        errors++;
      }
    }
    if (data.projectId !== void 0) {
      if (typeof data.projectId !== "string") {
        const err4 = { instancePath: instancePath + "/projectId", schemaPath: "#/properties/projectId/type", keyword: "type", params: { type: "string" }, message: "must be string" };
        if (vErrors === null) {
          vErrors = [err4];
        } else {
          vErrors.push(err4);
        }
        errors++;
      }
    }
  } else {
    const err5 = { instancePath, schemaPath: "#/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err5];
    } else {
      vErrors.push(err5);
    }
    errors++;
  }
  validate243.errors = vErrors;
  return errors === 0;
}
validate243.evaluated = { "props": true, "dynamicProps": false, "dynamicItems": false };
function validate282(data, { instancePath = "", parentData, parentDataProperty, rootData = data, dynamicAnchors = {} } = {}) {
  let vErrors = null;
  let errors = 0;
  const evaluated0 = validate282.evaluated;
  if (evaluated0.dynamicProps) {
    evaluated0.props = void 0;
  }
  if (evaluated0.dynamicItems) {
    evaluated0.items = void 0;
  }
  if (data && typeof data == "object" && !Array.isArray(data)) {
    if (data.editor === void 0) {
      const err0 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "editor" }, message: "must have required property 'editor'" };
      if (vErrors === null) {
        vErrors = [err0];
      } else {
        vErrors.push(err0);
      }
      errors++;
    }
    if (data.contexts === void 0) {
      const err1 = { instancePath, schemaPath: "#/required", keyword: "required", params: { missingProperty: "contexts" }, message: "must have required property 'contexts'" };
      if (vErrors === null) {
        vErrors = [err1];
      } else {
        vErrors.push(err1);
      }
      errors++;
    }
    for (const key0 in data) {
      if (!(key0 === "contexts" || key0 === "editor")) {
        const err2 = { instancePath, schemaPath: "#/additionalProperties", keyword: "additionalProperties", params: { additionalProperty: key0 }, message: "must NOT have additional properties" };
        if (vErrors === null) {
          vErrors = [err2];
        } else {
          vErrors.push(err2);
        }
        errors++;
      }
    }
    if (data.contexts !== void 0) {
      let data0 = data.contexts;
      if (Array.isArray(data0)) {
        const len0 = data0.length;
        for (let i0 = 0; i0 < len0; i0++) {
          if (!validate243(data0[i0], { instancePath: instancePath + "/contexts/" + i0, parentData: data0, parentDataProperty: i0, rootData, dynamicAnchors })) {
            vErrors = vErrors === null ? validate243.errors : vErrors.concat(validate243.errors);
            errors = vErrors.length;
          }
        }
      } else {
        const err3 = { instancePath: instancePath + "/contexts", schemaPath: "#/properties/contexts/type", keyword: "type", params: { type: "array" }, message: "must be array" };
        if (vErrors === null) {
          vErrors = [err3];
        } else {
          vErrors.push(err3);
        }
        errors++;
      }
    }
    if (data.editor !== void 0) {
      if (!validate56(data.editor, { instancePath: instancePath + "/editor", parentData: data, parentDataProperty: "editor", rootData, dynamicAnchors })) {
        vErrors = vErrors === null ? validate56.errors : vErrors.concat(validate56.errors);
        errors = vErrors.length;
      }
    }
  } else {
    const err4 = { instancePath, schemaPath: "#/type", keyword: "type", params: { type: "object" }, message: "must be object" };
    if (vErrors === null) {
      vErrors = [err4];
    } else {
      vErrors.push(err4);
    }
    errors++;
  }
  validate282.errors = vErrors;
  return errors === 0;
}
validate282.evaluated = { "props": true, "dynamicProps": false, "dynamicItems": false };
