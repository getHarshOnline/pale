// Portable source-occurrence pages over the retained SDK compilation.

/// Compiler-produced location and role, without source text.
#[wasm_bindgen(js_name = AuthoringOccurrence)]
pub struct WasmAuthoringOccurrence {
    inner: pale_sdk::AuthoringOccurrence,
}

#[wasm_bindgen(js_class = AuthoringOccurrence)]
impl WasmAuthoringOccurrence {
    /// Copy the logical source label and decoded field/index path.
    pub fn location(&self) -> WasmAuthoringSourceLocation {
        WasmAuthoringSourceLocation::from_api(self.inner.location())
    }

    /// Definition, reference, or projection role assigned by the compiler.
    pub fn role(&self) -> WasmAuthoringOccurrenceRole {
        self.inner.role().into()
    }
}

/// Opaque continuation for one compilation, full identity, and pair of limits.
///
/// The cursor retains its SDK compilation until freed or consumed. It cannot be
/// reconstructed from text and grants no permission to disclose source locations.
#[wasm_bindgen(js_name = AuthoringOccurrencePageCursor)]
pub struct WasmAuthoringOccurrencePageCursor {
    inner: pale_sdk::AuthoringOccurrencePageCursor,
}

/// Matching occurrences and scan progress from the retained compiler index.
///
/// An empty page can be incomplete. Row limits combine with one fixed ceiling
/// over copied source labels and field names.
#[wasm_bindgen(js_name = AuthoringOccurrencePage)]
pub struct WasmAuthoringOccurrencePage {
    inner: pale_sdk::AuthoringOccurrencePage,
}

#[wasm_bindgen(js_class = AuthoringOccurrencePage)]
impl WasmAuthoringOccurrencePage {
    /// Copy matching occurrences in compiler source/path order.
    pub fn occurrences(&self) -> Vec<WasmAuthoringOccurrence> {
        self.inner
            .occurrences()
            .iter()
            .cloned()
            .map(|inner| WasmAuthoringOccurrence { inner })
            .collect()
    }

    /// Index entries examined, including nonmatches.
    #[wasm_bindgen(getter)]
    pub fn inspected(&self) -> u16 {
        self.inner.inspected()
    }

    /// UTF-8 bytes copied into returned source labels and field names.
    #[wasm_bindgen(getter = copiedStringBytes)]
    pub fn copied_string_bytes(&self) -> u32 {
        self.inner.copied_string_bytes()
    }

    /// Whether the scan proved exhaustion of the retained index.
    #[wasm_bindgen(getter = isComplete)]
    pub fn is_complete(&self) -> bool {
        self.inner.is_complete()
    }

    /// Share the continuation, or return `undefined` after proven exhaustion.
    ///
    /// Each call creates an independently owned handle. Resuming consumes that
    /// handle; callers can retain another copy when they need to retry a query.
    pub fn continuation(&self) -> Option<WasmAuthoringOccurrencePageCursor> {
        self.inner
            .continuation()
            .cloned()
            .map(|inner| WasmAuthoringOccurrencePageCursor { inner })
    }
}

/// Preserve raw request limits before JavaScript-to-WASM integer conversion.
fn occurrence_page_limit(value: &JsValue) -> Result<u16, JsValue> {
    value
        .as_f64()
        .filter(|number| {
            number.is_finite()
                && number.fract() == 0.0
                && (0.0..=f64::from(u16::MAX)).contains(number)
        })
        .map(|number| number as u16)
        .ok_or_else(|| {
            javascript_error("authoring page limit must be an integer from 0 through 65535")
                .unwrap_or_else(std::convert::identity)
        })
}

/// Reject supplied wrappers whose native cursor is missing or already consumed.
///
/// Direct `Option<Cursor>` conversion treats a zero pointer as absence, while
/// generated `Option<JsValue>` glue also collapses null. Inspect the raw value
/// before using the pinned bindgen-generated consuming conversion. Its internal
/// [`wasm_bindgen::convert::TryFromJsValue`] API is unstable; upgrades must retain
/// the actual-JavaScript invalid-handle tests, not just compile this call.
fn occurrence_page_cursor(
    value: JsValue,
) -> Result<Option<WasmAuthoringOccurrencePageCursor>, JsValue> {
    use wasm_bindgen::convert::TryFromJsValue;

    if value.is_undefined() {
        return Ok(None);
    }
    WasmAuthoringOccurrencePageCursor::try_from_js_value(value)
        .map(Some)
        .map_err(|_| {
            javascript_error("continuation must be an unconsumed authoring occurrence cursor")
                .unwrap_or_else(std::convert::identity)
        })
}

#[wasm_bindgen(js_class = PortableWorkspaceCompilation)]
impl WasmPortableWorkspaceCompilation {
    /// Find occurrences of a compiler-produced identity without recompiling.
    ///
    /// Limits must be numbers representing integers from 0 through 65535. The
    /// SDK treats zero as 64 and caps values above 256. Resume on this exact
    /// compilation with the same identity and raw limits. Empty pages can continue.
    /// Omit the continuation for a first page; a supplied null or invalid handle
    /// is an error, not a request to restart.
    ///
    /// After receiver and identity validation, a supplied continuation is consumed
    /// even if limit validation or the SDK query fails. Obtain another handle
    /// from its page before retrying.
    ///
    /// # Errors
    ///
    /// Invalid limits or handles produce a host error, without coercion. An unlike
    /// continuation returns the canonical API diagnostic through the existing
    /// diagnostic projection.
    #[wasm_bindgen(js_name = authoringOccurrences)]
    pub fn authoring_occurrences(
        &self,
        identity: &WasmAuthoringIdentity,
        #[wasm_bindgen(unchecked_param_type = "number")] maximum_inspected: JsValue,
        #[wasm_bindgen(unchecked_param_type = "number")] maximum_returned: JsValue,
        #[wasm_bindgen(unchecked_optional_param_type = "AuthoringOccurrencePageCursor")]
        after: JsValue,
    ) -> Result<WasmAuthoringOccurrencePage, JsValue> {
        let after = occurrence_page_cursor(after)?;
        let request = pale_sdk::AuthoringOccurrencePageRequest::new(
            identity.inner.clone(),
            occurrence_page_limit(&maximum_inspected)?,
            occurrence_page_limit(&maximum_returned)?,
        );
        let request = match after {
            Some(cursor) => request.after(cursor.inner),
            None => request,
        };
        self.inner
            .authoring_occurrences(request)
            .map(|inner| WasmAuthoringOccurrencePage { inner })
            .map_err(diagnostic_error)
    }
}
