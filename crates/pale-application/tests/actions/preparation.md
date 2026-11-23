
The same prepared call cannot be consumed twice:

```compile_fail,E0382
use pale_application::actions::PreparedNativeAction;

fn execute_twice(call: PreparedNativeAction<'_>) {
    let _ = call.execute();
    let _ = call.execute(); // The first execution moved the call.
}
```

The call cannot be cloned to bypass consumption:

```compile_fail,E0599
use pale_application::actions::PreparedNativeAction;

fn duplicate(call: PreparedNativeAction<'_>) {
    let _ = call.clone();
}
```
