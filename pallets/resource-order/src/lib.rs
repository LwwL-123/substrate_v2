#![cfg_attr(not(feature = "std"), no_std)]


use frame_support::{dispatch::DispatchResult,
                    pallet_prelude::*, PalletId, traits::{Currency, ExistenceRequirement}};
use frame_support::sp_runtime::traits::Convert;
use frame_system::pallet_prelude::*;
use sp_core::Bytes;
use sp_runtime::traits::AccountIdConversion;
use sp_runtime::traits::Zero;
use sp_std::convert::TryInto;
use sp_std::vec::Vec;

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://substrate.dev/docs/en/knowledgebase/runtime/frame>

pub use pallet::*;
pub use primitives::p_provider::*;
pub use primitives::p_resource_order::*;

type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

const PALLET_ID: PalletId = PalletId(*b"ttchain!");


#[frame_support::pallet]
pub mod pallet {
    use super::*;

    /// Configure the pallet by specifying the parameters and types on which it depends.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// 支付费用和持有余额的货币。
        type Currency: Currency<Self::AccountId>;

        /// 订单费用接口
        type OrderInterface: OrderInterface<AccountId=Self::AccountId, BlockNumber=Self::BlockNumber>;

        /// 区块高度转数字
        type BlockNumberToNumber: Convert<Self::BlockNumber, u128> + Convert<u32, Self::BlockNumber>;

        /// 数字转金额
        type NumberToBalance: Convert<u128, BalanceOf<Self>>;
        /// 金额转换数字
        type BalanceToNumber: Convert<BalanceOf<Self>, u128>;

        /// 轮询间隔
        #[pallet::constant]
        type PollingInterval: Get<Self::BlockNumber>;

        /// 健康检查间隔
        #[pallet::constant]
        type HealthCheckInterval: Get<Self::BlockNumber>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub (super) trait Store)]
    pub struct Pallet<T>(_);

    /// 订单个数
    #[pallet::storage]
    #[pallet::getter(fn order_count)]
    pub(super) type OrderCount<T: Config> = StorageValue<_, u64, ValueQuery>;

    /// 资源订单信息
    #[pallet::storage]
    #[pallet::getter(fn resource_orders)]
    pub(super) type ResourceOrders<T: Config> = StorageMap<_, Twox64Concat, u64, ResourceOrder<T::AccountId, T::BlockNumber>, OptionQuery>;

    /// 租用协议个数
    #[pallet::storage]
    #[pallet::getter(fn agreement_count)]
    pub(super) type AgreementCount<T: Config> = StorageValue<_, u64, ValueQuery>;

    /// 租用协议信息
    #[pallet::storage]
    #[pallet::getter(fn rental_agreements)]
    pub(super) type RentalAgreements<T: Config> = StorageMap<_, Twox64Concat, u64, RentalAgreement<T::AccountId, T::BlockNumber>, OptionQuery>;

    /// 资源对应的租赁协议 (资源订单号，租用协议号)
    #[pallet::storage]
    #[pallet::getter(fn order_info)]
    pub(super) type OrderInfo<T: Config> = StorageMap<_, Twox64Concat, u64, u64, OptionQuery>;

    /// 租用方对应的协议列表
    #[pallet::storage]
    #[pallet::getter(fn resources)]
    pub(super) type Resources<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, Vec<u64>, ValueQuery>;

    /// 质押
    #[pallet::storage]
    #[pallet::getter(fn staking)]
    pub(super) type Staking<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, StakingAmount, OptionQuery>;

    /// 迭代器
    #[pallet::storage]
    #[pallet::getter(fn iter)]
    pub(super) type Iter<T: Config> = StorageValue<_, u64, OptionQuery>;


    // Pallets use events to inform users when important changes are made.
    // https://substrate.dev/docs/en/knowledgebase/runtime/events
    #[pallet::event]
    #[pallet::metadata(T::AccountId = "AccountId")]
    #[pallet::generate_deposit(pub (super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// 创建订单成功
        /// [账户,订单号,租用资源号,租用时长(h),用户公钥]
        CreateOrderSuccess(T::AccountId, u64, u64, u32, Bytes),

        /// 订单续费成功
        /// [账户,订单号,租用资源号,租用时长(h)]
        ReNewOrderSuccess(T::AccountId, u64, u64, u32),

        /// 订单执行成功
        /// [账户,订单号,租用资源号,租用协议号]
        OrderExecSuccess(T::AccountId, u64, u64, u64),

        /// 健康检查上报成功
        /// [账户,协议号,上报协议的区块号]
        HealthCheckSuccess(T::AccountId, u64, T::BlockNumber),

        /// 质押金额成功
        StakingSuccess(T::AccountId, BalanceOf<T>),

        /// 取回质押金额成功
        WithdrawStakingSuccess(T::AccountId, BalanceOf<T>),

        /// 取回租用奖励金额成功
        /// [账户,协议号,金额]
        WithdrawRentalAmountSuccess(T::AccountId, u64, BalanceOf<T>),

        /// 取回惩罚金额成功
        /// [账户,协议号,金额]
        WithdrawFaultExcutionSuccess(T::AccountId, u64, BalanceOf<T>),

        /// 领回未开始订单金额成功
        /// [账户,订单号,金额]
        WithdrawLockedOrderPriceSuccess(T::AccountId, u64, BalanceOf<T>),

        /// 过期协议删除成功
        /// [协议号]
        ExpiredAgreementDeletedSuccessfully(u64),

        /// 惩罚协议执行成功
        PenaltyAgreementExcutionSuccess(u64),

    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(now: T::BlockNumber) -> Weight {
            // 检查迭代器是否存在
            if Iter::<T>::get() != None {
                // 获取这次要迭代的开始索引
                let index = Iter::<T>::get().unwrap();
                Self::do_health_check(index, now);
            }

            if (now % T::PollingInterval::get()).is_zero() {
                // 初始化迭代器
                Iter::<T>::put(0);
            }
            0
        }
    }

    // Errors inform users that something went wrong.
    #[pallet::error]
    pub enum Error<T> {
        /// 用户余额不足
        InsufficientCurrency,
        /// 资源已被租用
        ResourceHasBeenRented,
        /// 超出可租用时长
        ExceedTheRentablTime,
        /// 订单拥有者不是本人
        OrderNotOwnedByYou,
        /// 协议拥有者不是本人
        ProtocolNotOwnedByYou,
        /// 订单不存在
        OrderDoesNotExist,
        /// 订单状态错误
        OrderStatusError,
        /// 协议不存在
        ProtocolDoesNotExist,
        /// 质押金额不足
        InsufficientStaking,
        /// 质押不存在
        StakingNotExist,
        /// 资源出租时间不足
        InsufficientTimeForResource,
        /// 领取失败
        FailedToWithdraw,
    }

    // Dispatchable functions allows users to interact with the pallet and invoke state changes.
    // These functions materialize as "extrinsics", which are often compared to transactions.
    // Dispatchable functions must be annotated with a weight and must return a DispatchResult.
    #[pallet::call]
    impl<T: Config> Pallet<T> {

        /// 创建订单
        /// [资源号,租赁时长(小时),公钥]
        #[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
        pub fn create_order_info(
            origin: OriginFor<T>,
            resource_index: u64,
            rent_duration: u32,
            public_key: Bytes,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            // 获取资源信息
            let mut resource_info = T::OrderInterface::get_computing_resource_info(resource_index);
            // 判断资源是否被租用
            ensure!(resource_info.status == ResourceStatus::Unused, Error::<T>::ResourceHasBeenRented);

            // 获得当前块高
            let block_number = <frame_system::Pallet<T>>::block_number();
            // 计算持续的区块
            let rent_blocks = TryInto::<T::BlockNumber>::try_into(rent_duration * 600).ok().unwrap();
            // 判断是否超出可租用时长
            ensure!(block_number + rent_blocks < resource_info.rental_info.end_of_rent,Error::<T>::ExceedTheRentablTime);

            // 获取订单长度
            let order_index = OrderCount::<T>::get();
            // 获得订单单价
            let price = resource_info.rental_info.rent_unit_price;
            // 计算订单价格
            let order_price = price * rent_duration as u128;
            // 创建租用者
            let customer = TenantInfo::new(who.clone(), public_key.clone());
            // 转账到资金池中
            T::Currency::transfer(&who.clone(), &Self::order_pool(), T::NumberToBalance::convert(order_price), ExistenceRequirement::AllowDeath)?;
            // 创建订单
            let order = ResourceOrder::new(
                order_index,
                customer,
                order_price,
                resource_index,
                block_number,
                rent_blocks,
            );
            // 资源状态由未使用改为已锁定
            resource_info.update_status(ResourceStatus::Locked);

            // 保存资源状态
            T::OrderInterface::update_computing_resource(resource_index, resource_info);
            // 将订单加入订单集合中
            ResourceOrders::<T>::insert(order_index, order.clone());
            // 订单长度+1
            OrderCount::<T>::put(order_index + 1);

            Self::deposit_event(Event::CreateOrderSuccess(who, order_index, resource_index, rent_duration, public_key));
            Ok(())
        }


        /// 订单执行
        #[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
        pub fn order_exec(
            origin: OriginFor<T>,
            order_index: u64,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // 判断订单是否存在
            ensure!(ResourceOrders::<T>::contains_key(order_index),Error::<T>::OrderDoesNotExist);
            // 获取订单详情
            let mut order = ResourceOrders::<T>::get(order_index).unwrap();
            // 判断订单状态
            ensure!(order.status == OrderStatus::Pending,Error::<T>::OrderStatusError);

            // 获取资源详情
            let mut resource_info = T::OrderInterface::get_computing_resource_info(order.resource_index);
            // 判断是否是本人
            ensure!(who.clone() == resource_info.account_id,Error::<T>::OrderNotOwnedByYou);


            // 获取订单金额
            let order_price = order.price;
            // 获取质押信息
            ensure!(Staking::<T>::contains_key(who.clone()),Error::<T>::InsufficientStaking);
            let mut staking_info = Staking::<T>::get(who.clone()).unwrap();
            // 判断质押金是否充足,并锁定金额
            ensure!(&staking_info.lock_amount(order_price),Error::<T>::InsufficientStaking);

            // 获得当前块高
            let block_number = <frame_system::Pallet<T>>::block_number();
            // 获取资源号
            let resource_index = order.resource_index;

            // 查询订单是否有对应协议
            if OrderInfo::<T>::contains_key(order_index) {
                // 查询资源协议号
                let agreement_count = OrderInfo::<T>::get(order_index).unwrap();
                // 查询协议
                let mut agreement = RentalAgreements::<T>::get(agreement_count).unwrap();
                // 获得订单持续时间
                let duration = order.rent_duration;
                // 协议续费
                agreement.renew(order_price, duration, resource_info);
                // 订单状态变为已完成
                order.finish_order();

                // 将协议加入租用协议集合中
                RentalAgreements::<T>::insert(agreement_count, agreement.clone());
                // 保存订单
                ResourceOrders::<T>::insert(order_index, order.clone());
                // 保存质押
                Staking::<T>::insert(who.clone(), staking_info);

                Self::deposit_event(Event::OrderExecSuccess(who.clone(), order_index, resource_index, agreement_count));
            } else {
                // 获取协议号
                let agreement_count = AgreementCount::<T>::get();
                // 判断资源是否被锁定
                ensure!(resource_info.status == ResourceStatus::Locked, Error::<T>::ResourceHasBeenRented);
                // 获取peerId
                let peer_id = resource_info.peer_id.clone();
                // 结束区块
                let end = block_number + order.rent_duration;
                // 创建租用协议
                let agreement = RentalAgreement::new(
                    agreement_count,
                    who.clone(),
                    order.clone().tenant_info,
                    peer_id,
                    resource_index,
                    resource_info.config.clone(),
                    resource_info.rental_info.clone(),
                    order_price,
                    order.price,
                    0,
                    0,
                    block_number,
                    end,
                    block_number,
                );

                // 订单状态变为已完成
                order.finish_order();
                // 将用户和协议号关联
                Self::do_insert_resource(agreement.tenant_info.account_id.clone(), agreement_count);

                // 协议号+1
                AgreementCount::<T>::put(agreement_count + 1);
                // 将协议和订单关联
                OrderInfo::<T>::insert(order_index, agreement_count);
                // 将协议加入租用协议集合中
                RentalAgreements::<T>::insert(agreement_count, agreement.clone());


                // 保存订单
                ResourceOrders::<T>::insert(order_index, order.clone());
                // 保存质押
                Staking::<T>::insert(who.clone(), staking_info);


                // 资源状态由已锁定改为使用中
                resource_info.update_status(ResourceStatus::Inuse);
                // 保存资源状态
                T::OrderInterface::update_computing_resource(resource_index, resource_info.clone());

                Self::deposit_event(Event::OrderExecSuccess(who.clone(), order_index, resource_index, agreement_count));
            }


            Ok(())
        }

        /// 协议资源心跳上报
        #[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
        pub fn heartbeat(
            origin: OriginFor<T>,
            agreement_index: u64,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            // 获取协议
            ensure!(RentalAgreements::<T>::contains_key(agreement_index),Error::<T>::ProtocolDoesNotExist);
            let mut agreement = RentalAgreements::<T>::get(agreement_index).unwrap();
            // 判断是否是本人
            ensure!(who.clone() == agreement.provider,Error::<T>::ProtocolNotOwnedByYou);
            // 获得当前块高
            let block_number = <frame_system::Pallet<T>>::block_number();

            // 执行协议，现行释放金额
            agreement.execution(&block_number);

            // 保存协议
            RentalAgreements::<T>::insert(agreement_index, agreement.clone());

            Self::deposit_event(Event::HealthCheckSuccess(who.clone(), agreement_index, block_number));
            Ok(())
        }

        /// 质押
        #[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
        pub fn staking_amount(
            origin: OriginFor<T>,
            bond_price: BalanceOf<T>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // 转账
            T::Currency::transfer(&who.clone(), &Self::staking_pool(), bond_price, ExistenceRequirement::AllowDeath)?;

            // 如果有质押
            if Staking::<T>::contains_key(&who) {
                // 获取质押详情
                let mut staking_info = Staking::<T>::get(who.clone()).unwrap();
                // 计算新的质押总金额
                let price = T::BalanceToNumber::convert(bond_price);
                // 质押金额
                staking_info.staking_amount(price);
                // 保存质押详情
                Staking::<T>::insert(who.clone(), staking_info);
            } else {
                // 新增质押详情
                Staking::<T>::insert(who.clone(), StakingAmount {
                    amount: T::BalanceToNumber::convert(bond_price),
                    active_amount: T::BalanceToNumber::convert(bond_price),
                    lock_amount: 0,
                });
            }

            Self::deposit_event(Event::StakingSuccess(who.clone(), bond_price));
            Ok(())
        }


        /// 取回质押
        #[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
        pub fn withdraw_amount(
            origin: OriginFor<T>,
            price: BalanceOf<T>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            // 判断是否存在质押
            ensure!(Staking::<T>::contains_key(who.clone()),Error::<T>::StakingNotExist);

            let mut staking = Staking::<T>::get(who.clone()).unwrap();

            // 取回金额
            ensure!(&staking.withdraw_amount(T::BalanceToNumber::convert(price)),Error::<T>::InsufficientStaking);
            // 转账
            T::Currency::transfer(&Self::staking_pool(), &who.clone(), price, ExistenceRequirement::AllowDeath)?;

            // 保存质押
            Staking::<T>::insert(who.clone(), staking);

            Self::deposit_event(Event::WithdrawStakingSuccess(who.clone(), price));

            Ok(())
        }


        /// 取回租用奖励金额
        #[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
        pub fn withdraw_rental_amount(
            origin: OriginFor<T>,
            agreement_index: u64,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // 获取协议
            ensure!(RentalAgreements::<T>::contains_key(agreement_index),Error::<T>::ProtocolDoesNotExist);
            let mut agreement = RentalAgreements::<T>::get(agreement_index).unwrap();
            // 判断是否是本人
            ensure!(who.clone() == agreement.provider,Error::<T>::ProtocolNotOwnedByYou);
            // 获取可领取金额
            let price = T::NumberToBalance::convert(agreement.withdraw());
            // 转账领取金额
            T::Currency::transfer(&Self::order_pool(), &who.clone(), price, ExistenceRequirement::AllowDeath)?;

            // 保存协议
            RentalAgreements::<T>::insert(agreement_index, agreement.clone());

            Self::deposit_event(Event::WithdrawRentalAmountSuccess(who.clone(), agreement_index, price));
            Ok(())
        }

        /// 取回协议的惩罚金额
        #[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
        pub fn withdraw_fault_excution(
            origin: OriginFor<T>,
            agreement_index: u64,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // 获取协议
            ensure!(RentalAgreements::<T>::contains_key(agreement_index),Error::<T>::ProtocolDoesNotExist);
            let mut agreement = RentalAgreements::<T>::get(agreement_index).unwrap();
            // 判断是否是用户
            ensure!(who.clone() == agreement.tenant_info.account_id,Error::<T>::ProtocolNotOwnedByYou);
            // 获取可领取金额
            let price = T::NumberToBalance::convert(agreement.withdraw_penalty());
            // 转账领取金额
            T::Currency::transfer(&Self::order_pool(), &who.clone(), price, ExistenceRequirement::AllowDeath)?;

            // 保存协议
            RentalAgreements::<T>::insert(agreement_index, agreement.clone());

            Self::deposit_event(Event::WithdrawFaultExcutionSuccess(who.clone(), agreement_index, price));
            Ok(())
        }

        /// 取回未执行订单的金额
        #[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
        pub fn withdraw_locked_order_price(
            origin: OriginFor<T>,
            order_index: u64,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            // 检查是否存在订单
            ensure!(ResourceOrders::<T>::contains_key(order_index),Error::<T>::OrderDoesNotExist);
            // 获取订单
            let mut order = ResourceOrders::<T>::get(order_index).unwrap();
            // 判断是否是用户
            ensure!(who.clone() == order.tenant_info.account_id,Error::<T>::ProtocolNotOwnedByYou);
            // 获取资源
            let mut resource = T::OrderInterface::get_computing_resource_info(order.resource_index);
            // 获取订单金额
            let price = T::NumberToBalance::convert(order.price);

            // 检查订单状态
            if order.status == OrderStatus::Pending && resource.status == ResourceStatus::Locked {

                // 取消订单
                order.cancel_order();
                // 改变资源状态为未使用
                resource.status = ResourceStatus::Unused;
                // 取回金额
                T::Currency::transfer(&Self::order_pool(), &who.clone(), price, ExistenceRequirement::AllowDeath)?;

                // 保存订单
                ResourceOrders::<T>::insert(order_index, order);
                // 保存资源状态
                T::OrderInterface::update_computing_resource(resource.index, resource);

                Self::deposit_event(Event::WithdrawLockedOrderPriceSuccess(who.clone(), order_index, price));
            } else {
                return Err(Error::<T>::FailedToWithdraw)?;
            }

            Ok(())
        }

        /// 协议续费
        #[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
        pub fn renew_agreement(
            origin: OriginFor<T>,
            agreement_index: u64,
            duration: u32,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // 获取协议
            ensure!(RentalAgreements::<T>::contains_key(agreement_index),Error::<T>::ProtocolDoesNotExist);
            let agreement = RentalAgreements::<T>::get(agreement_index).unwrap();

            // 获取资源号
            let resource_index = agreement.resource_index;
            // 获取资源信息
            let resource_info = T::OrderInterface::get_computing_resource_info(resource_index);

            // 获得当前块高
            let block_number = <frame_system::Pallet<T>>::block_number();
            // 获取资源结束时间
            let end_resource = resource_info.rental_info.end_of_rent;
            // 获取出租区块
            let rent_duration = T::BlockNumberToNumber::convert(duration * 600);
            ensure!(rent_duration + block_number < end_resource,Error::<T>::InsufficientTimeForResource);
            // 计算新订单价格
            let price = resource_info.rental_info.rent_unit_price * duration as u128;
            // 转账到资金池中
            T::Currency::transfer(&who.clone(), &Self::order_pool(), T::NumberToBalance::convert(price), ExistenceRequirement::AllowDeath)?;

            // 获取订单长度
            let order_index = OrderCount::<T>::get();
            // 获得当前块高
            let block_number = <frame_system::Pallet<T>>::block_number();

            let order = ResourceOrder::new(
                order_index,
                agreement.tenant_info.clone(),
                price,
                resource_index,
                block_number,
                rent_duration,
            );

            // 将订单加入订单集合中
            ResourceOrders::<T>::insert(order_index, order.clone());
            // 将协议和订单关联
            OrderInfo::<T>::insert(order_index, agreement_index);
            // 订单长度+1
            OrderCount::<T>::put(order_index + 1);

            Self::deposit_event(Event::ReNewOrderSuccess(who.clone(), order_index, resource_index, duration));
            Ok(())
        }
    }
}


impl<T: Config> Pallet<T> {
    /// StakingPod
    pub fn staking_pool() -> T::AccountId { PALLET_ID.into_sub_account(b"staking") }
    /// StoragePod
    pub fn order_pool() -> T::AccountId { PALLET_ID.into_sub_account(b"order") }

    // 将用户和协议号关联
    pub fn do_insert_resource(who: T::AccountId, agreement_count: u64) {
        // 检测是否存在用户的协议
        if !Resources::<T>::contains_key(who.clone()) {
            let mut vec = Vec::new();
            vec.push(agreement_count);

            Resources::<T>::insert(who.clone(), vec);
        }

        let mut vec = Resources::<T>::get(who.clone());
        vec.push(agreement_count);
        Resources::<T>::insert(who, vec);
    }

    // 健康检查以及订单过期检查
    pub fn do_health_check(index: u64, now: T::BlockNumber) {
        // 获取协议数量
        let agreement_count = AgreementCount::<T>::get();
        // 本次遍历的订单数量
        let num: u64;
        if index + 400 < agreement_count {
            num = index + 400;
            Iter::<T>::put(num);
        } else {
            num = agreement_count;
            Iter::<T>::kill();
        }

        // 遍历协议
        for i in index..num {
            // 判断协议是否存在
            if RentalAgreements::<T>::contains_key(i) {
                let mut agreement = RentalAgreements::<T>::get(i).unwrap();
                // 检查协议是否过期
                if agreement.end < now {
                    // 如果订单金额已清算完成
                    if agreement.lock_price == 0 && agreement.penalty_amount == 0 && agreement.receive_amount == 0 {
                        RentalAgreements::<T>::remove(i);
                        // 获取provider的质押
                        let mut staking = Staking::<T>::get(agreement.provider.clone()).unwrap();
                        // 解除质押
                        staking.unlock_amount(agreement.price);
                        // 保存质押
                        Staking::<T>::insert(agreement.provider, staking);

                        Self::deposit_event(Event::ExpiredAgreementDeletedSuccessfully(i));
                    }
                } else {
                    // 检查协议是否上报健康检查
                    if now - agreement.calculation > T::HealthCheckInterval::get() {
                        // 执行惩罚协议
                        agreement.fault_excution();


                        // 保存协议
                        RentalAgreements::<T>::insert(i, agreement);
                        Self::deposit_event(Event::PenaltyAgreementExcutionSuccess(i));
                    }
                }
            }
        }
    }
}




