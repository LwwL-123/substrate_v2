#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{dispatch::DispatchResult,
                    pallet_prelude::*, traits::{Currency}};
use frame_support::sp_runtime::traits::Convert;
use frame_system::pallet_prelude::*;
use sp_std::convert::TryInto;
use sp_std::vec::Vec;

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://substrate.dev/docs/en/knowledgebase/runtime/frame>

pub use pallet::*;
pub use primitives::p_resource_order::*;
pub use primitives::p_provider::*;

type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;


#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use primitives::Balance;

    /// Configure the pallet by specifying the parameters and types on which it depends.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// 支付费用和持有余额的货币。
        type Currency: Currency<Self::AccountId>;

        /// 金额转换数字
        type BalanceToNumber: Convert<BalanceOf<Self>, u128>;

        // /// 资源provider接口
        // type ProviderInterface: ProviderInterface<BlockNumber=Self::BlockNumber, AccountId=Self::AccountId>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub (super) trait Store)]
    pub struct Pallet<T>(_);

    /// 资源信息
    #[pallet::storage]
    #[pallet::getter(fn resource)]
    pub(super) type Resources<T: Config> = StorageMap<_, Twox64Concat, u64, ComputingResource<T::BlockNumber, T::AccountId>, OptionQuery>;


    /// 资源个数
    #[pallet::storage]
    #[pallet::getter(fn resource_count)]
    pub(super) type ResourceCount<T: Config> = StorageValue<_, u64, ValueQuery>;


    /// 资源提供者和资源的关联
    #[pallet::storage]
    #[pallet::getter(fn provider)]
    pub(super) type Provider<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, Vec<u64>, OptionQuery>;

    // Pallets use events to inform users when important changes are made.
    // https://substrate.dev/docs/en/knowledgebase/runtime/events
    #[pallet::event]
    #[pallet::metadata(T::AccountId = "AccountId")]
    #[pallet::generate_deposit(pub (super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// 注册资源成功 [accountId, index, peerId, cpu, memory, system, cpu_model, price_hour, rent_duration_hour]
        RegisterResourceSuccess(T::AccountId, u64, Vec<u8>, u64, u64, Vec<u8>, Vec<u8>, Balance, u32),
        /// 修改资源单价成功 [accountId, index, balance]
        ModifyResourceUnitPrice(T::AccountId, u64, u128),
        /// 删除成功
        RemoveSuccess(T::AccountId, u64),
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(_now: T::BlockNumber) -> Weight {
            0
        }
    }

    // Errors inform users that something went wrong.
    #[pallet::error]
    pub enum Error<T> {
        /// 资源不存在
        ResourceNotFound,
        /// 非法请求
        IllegalRequest,
    }

    // Dispatchable functions allows users to interact with the pallet and invoke state changes.
    // These functions materialize as "extrinsics", which are often compared to transactions.
    // Dispatchable functions must be annotated with a weight and must return a DispatchResult.
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// 注册资源
        #[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
        pub fn register_resource(
            account_id: OriginFor<T>,
            peer_id: Vec<u8>,
            cpu: u64,
            memory: u64,
            system: Vec<u8>,
            cpu_model: Vec<u8>,
            price: BalanceOf<T>,
            rent_duration_hour: u32,
        ) -> DispatchResult {
            let who = ensure_signed(account_id)?;
            let index = ResourceCount::<T>::get();

            let resource_config =
                ResourceConfig::new(cpu.clone(), memory.clone(),
                                    system.clone(), cpu_model.clone());

            let statistics =
                ResourceRentalStatistics::new(0, 0, 0, 0);

            // 获得当前块高
            let block_number = <frame_system::Pallet<T>>::block_number();
            // 计算持续的块
            let rent_blocks =
                TryInto::<T::BlockNumber>::try_into(&rent_duration_hour * 600).ok().unwrap();
            // 计算结束的块号
            let end_of_block = block_number + rent_blocks;

            let resource_rental_info =
                ResourceRentalInfo::new(T::BalanceToNumber::convert(price.clone()), rent_blocks, end_of_block);

            let computing_resource = ComputingResource::new(
                index, who.clone(), peer_id.clone(), resource_config,
                statistics, resource_rental_info,
                ResourceStatus::Unused,
            );

            //save
            Resources::<T>::insert(index, computing_resource.clone());
            //增加总数
            ResourceCount::<T>::set(index + 1);
            //更新发布者关联资源
            if !Provider::<T>::contains_key(who.clone()) {
                // 初始化
                let vec: Vec<u64> = Vec::new();
                Provider::<T>::insert(who.clone(), vec);
            }
            ensure!(Provider::<T>::contains_key(who.clone()),Error::<T>::ResourceNotFound);
            let mut resources = Provider::<T>::get(who.clone()).unwrap();
            resources.push(index);
            Provider::<T>::insert(who.clone(), resources);

            Self::deposit_event(Event::RegisterResourceSuccess(who, index, peer_id, cpu, memory, system, cpu_model, T::BalanceToNumber::convert(price), rent_duration_hour));

            Ok(())
        }

        /// 修改资源单价
        #[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
        pub fn modify_rent_unit_price(
            account_id: OriginFor<T>,
            index: u64,
            unit_price: BalanceOf<T>,
        ) -> DispatchResult {
            let who = ensure_signed(account_id)?;
            //查询并修改
            ensure!(Resources::<T>::contains_key(index),Error::<T>::ResourceNotFound);
            let mut resource =
                Self::get_computing_resource_info(index.clone()).unwrap();

            resource.update_rental_unit_price(T::BalanceToNumber::convert(unit_price.clone()));
            Resources::<T>::insert(&index, resource);

            Self::deposit_event(Event::ModifyResourceUnitPrice(who, index, T::BalanceToNumber::convert(unit_price)));

            Ok(())
        }

        /// 删除资源
        #[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
        pub fn remove_resource(
            account_id: OriginFor<T>,
            index: u64,
        ) -> DispatchResult {
            let who = ensure_signed(account_id)?;
            let resource =
                Self::get_computing_resource_info(index.clone()).unwrap();

            ensure!(resource.account_id == who.clone(), Error::<T>::ResourceNotFound);
            //删除
            Resources::<T>::remove(&index);

            Self::deposit_event(Event::RemoveSuccess(who, index));

            Ok(())
        }
    }
}


impl<T: Config> Pallet<T> {
    /// 根据index查询资源
    fn get_computing_resource_info(index: u64) -> Result<ComputingResource<T::BlockNumber, T::AccountId>, Error<T>> {
        ensure!(Resources::<T>::contains_key(index),Error::<T>::ResourceNotFound);
        let res = Resources::<T>::get(index).unwrap();
        Ok(res)
    }

    /// 修改资源
    fn update_computing_resource(index: u64,
                                 resource: ComputingResource<T::BlockNumber,
                                     T::AccountId>,
    ) -> Result<(), Error<T>> {
        ensure!(Resources::<T>::contains_key(index),Error::<T>::ResourceNotFound);

        Resources::<T>::insert(index, resource);
        Ok(())
    }
}

impl<T: Config> OrderInterface for Pallet<T> {
    type AccountId = T::AccountId;
    type BlockNumber = T::BlockNumber;


    fn get_computing_resource_info(index: u64) -> ComputingResource<Self::BlockNumber, Self::AccountId> {
        Self::get_computing_resource_info(index).unwrap()
    }

    fn update_computing_resource(index: u64, resource_info: ComputingResource<Self::BlockNumber, Self::AccountId>) {
        Self::update_computing_resource(index, resource_info).ok();
    }
}

