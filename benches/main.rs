mod ljr;
mod lua;
mod mlua;

use criterion::{Criterion, criterion_group, criterion_main};

fn bench_do_string_primitive(c: &mut Criterion) {
    let mut group = c.benchmark_group("do_string_primitive");
    group.bench_function("lua_do_string_primitive", |b| {
        b.iter(|| lua::do_string_primitive())
    });
    group.bench_function("mlua_do_string_primitive", |b| {
        b.iter(|| mlua::do_string_primitive())
    });
    group.bench_function("ljr_do_string_primitive", |b| {
        b.iter(|| ljr::do_string_primitive())
    });
    group.finish();
}

fn bench_call_fn_primitive(c: &mut Criterion) {
    let mut group = c.benchmark_group("call_fn_primitive");
    group.bench_function("lua_call_fn_primitive", |b| {
        b.iter(|| lua::call_fn_primitive())
    });
    group.bench_function("mlua_call_fn_primitive", |b| {
        b.iter(|| mlua::call_fn_primitive())
    });
    group.bench_function("ljr_call_fn_primitive", |b| {
        b.iter(|| ljr::call_fn_primitive())
    });
    group.finish();
}

fn bench_call_fn_string(c: &mut Criterion) {
    let mut group = c.benchmark_group("call_fn_string");
    group.bench_function("lua_call_fn_string", |b| b.iter(|| lua::call_fn_string()));
    group.bench_function("mlua_call_fn_string", |b| b.iter(|| mlua::call_fn_string()));
    group.bench_function("ljr_call_fn_string", |b| b.iter(|| ljr::call_fn_string()));
    group.finish();
}

fn bench_userdata_simple(c: &mut Criterion) {
    let mut group = c.benchmark_group("userdata_simple");
    group.bench_function("lua_userdata_simple", |b| b.iter(|| lua::userdata_simple()));
    group.bench_function("mlua_userdata_simple", |b| {
        b.iter(|| mlua::userdata_simple())
    });
    group.bench_function("ljr_userdata_simple", |b| b.iter(|| ljr::userdata_simple()));
    group.finish();
}

fn bench_userdata_mut(c: &mut Criterion) {
    let mut group = c.benchmark_group("userdata_mut");
    group.bench_function("lua_userdata_mut", |b| b.iter(|| lua::userdata_mut()));
    group.bench_function("mlua_userdata_mut", |b| b.iter(|| mlua::userdata_mut()));
    group.bench_function("ljr_userdata_mut", |b| b.iter(|| ljr::userdata_mut()));
    group.finish();
}

criterion_group!(
    benches,
    bench_do_string_primitive,
    bench_call_fn_primitive,
    bench_call_fn_string,
    bench_userdata_simple,
    bench_userdata_mut
);

criterion_main!(benches);
