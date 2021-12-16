//! Benchmarking setup for pallet-template


use super::*;

use frame_system::RawOrigin;
use frame_benchmarking::{benchmarks, whitelisted_caller, impl_benchmark_test_suite};
#[allow(unused)]
use crate::Pallet as Template;
use sp_core::Bytes;
use frame_benchmarking::vec;


benchmarks! {
	//这将测量 [1..100] 范围内 b 的 `do_something` 的执行时间。
	create_order_info {
		let caller: T::AccountId = whitelisted_caller();

		let resource_index:u64 = 0;
		let rent_duration:u32 = 100;
		let public_key = Bytes(vec![1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30]);
	}: _(RawOrigin::Signed(caller), resource_index,rent_duration,public_key)
	// verify {
	// 	let resource_info = T::OrderInterface::get_computing_resource_info(resource_index).unwrap();
	// 	assert!(resource_info.status == ResourceStatus::Unused);
    //
	// }



	// set_dummy_benchmark {
	// 	// This is the benchmark setup phase
	// 	let b in 1 .. 1000;
	// }: set_dummy(RawOrigin::Root, b.into()) // 执行阶段只是运行 `set_dummy` 外部调用
	// verify {
	// 	// 这是可选的基准验证阶段，测试某些状态。
	// 	assert_eq!(Pallet::<T>::dummy(), Some(b.into()))
	// }
	//
	// accumulate_dummy {
	// 	let b in 1 .. 1000;
	// 	// 调用者帐户被基准测试宏列入数据库读写白名单.
	// 	let caller: T::AccountId = whitelisted_caller();
	// }: _(RawOrigin::Signed(caller), b.into())
}

impl_benchmark_test_suite!(
	Template,
	crate::mock::new_test_ext(),
	crate::mock::Test,
);