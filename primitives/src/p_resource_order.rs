use codec::{Decode, Encode};
use frame_support::Parameter;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_core::Bytes;
use sp_debug_derive::RuntimeDebug;
use sp_runtime::traits::AtLeast32BitUnsigned;
use sp_std::convert::TryInto;
use sp_std::vec::Vec;

use crate::p_provider::{ComputingResource, ResourceConfig, ResourceRentalInfo};
use sp_core::sp_std::time::Duration;

/// 资源订单
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct ResourceOrder<AccountId, BlockNumber> {
    /// 订单id索引
    pub index: u64,
    /// 租用者信息
    pub tenant_info: TenantInfo<AccountId>,
    /// 订单金额
    pub price: u128,
    /// 资源索引
    pub resource_index: u64,
    /// 创建时区块
    pub create: BlockNumber,
    /// 租用时长
    pub rent_duration: BlockNumber,
    /// 时间戳
    pub time: Duration,
    /// 订单状态
    pub status: OrderStatus,
    /// 协议号
    pub agreement_index: Option<u64>,
}

/// 租用者信息
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct TenantInfo<AccountId> {
    /// 租用者信息
    pub account_id: AccountId,
    /// 租用者公钥
    pub public_key: Bytes,
}

/// 租用协议
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct RentalAgreement<AccountId, BlockNumber>
    where BlockNumber: Parameter + AtLeast32BitUnsigned
{
    ///协议id索引
    pub index: u64,
    /// 出租人
    pub provider: AccountId,
    /// 租用者信息
    pub tenant_info: TenantInfo<AccountId>,
    /// 计算资源链接ID
    pub peer_id: Vec<u8>,
    /// 资源索引
    pub resource_index: u64,
    /// 资源配置
    pub config: ResourceConfig,
    /// 资源出租信息
    pub rental_info: ResourceRentalInfo<BlockNumber>,
    /// 租用金额
    pub price: u128,
    /// 锁定金额
    pub lock_price: u128,
    /// 惩罚金额
    pub penalty_amount: u128,
    /// 领取金额
    pub receive_amount: u128,
    /// 开始区块
    pub start: BlockNumber,
    /// 协议到期区块
    pub end: BlockNumber,
    /// 计算区块
    pub calculation: BlockNumber,
    /// 时间戳
    pub time: Duration,
}

/// 质押金额
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct StakingAmount {
    /// 质押金额
    pub amount: u128,
    /// 活跃金额
    pub active_amount: u128,
    /// 锁定的金额
    pub lock_amount: u128,
}


#[derive(Encode, Decode, RuntimeDebug, PartialEq, Eq, Copy, Clone)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum OrderStatus {
    /// 待处理.
    Pending,
    /// 已完成.
    Finished,
    /// 已取消.
    Canceled,
}

impl<AccountId, BlockNumber> ResourceOrder<AccountId, BlockNumber> {
    /// 创建新资源订单
    pub fn new(index: u64, tenant_info: TenantInfo<AccountId>, price: u128, resource_index: u64, create: BlockNumber, rent_duration: BlockNumber,time:Duration) -> Self {
        ResourceOrder {
            index,
            tenant_info,
            price,
            resource_index,
            create,
            rent_duration,
            time,
            status: OrderStatus::Pending,
            agreement_index: None
        }
    }

    /// 创建续费订单
    pub fn renew(index: u64, tenant_info: TenantInfo<AccountId>, price: u128, resource_index: u64, create: BlockNumber, rent_duration: BlockNumber,time:Duration,agreement_index:Option<u64>) -> Self {
        ResourceOrder {
            index,
            tenant_info,
            price,
            resource_index,
            create,
            rent_duration,
            time,
            status: OrderStatus::Pending,
            agreement_index,
        }
    }

    /// 是否是续费订单
    pub fn is_renew_order(self) -> bool{
        match self.agreement_index {
            Some(_) => true,
            None => false
        }
    }

    /// 完成订单
    pub fn finish_order(&mut self) { self.status = OrderStatus::Finished }

    /// 取消订单
    pub fn cancel_order(&mut self) { self.status = OrderStatus::Canceled }
}

impl<AccountId, BlockNumber> RentalAgreement<AccountId, BlockNumber>
    where BlockNumber: Parameter + AtLeast32BitUnsigned
{
    pub fn new(index: u64, provider: AccountId, tenant_info: TenantInfo<AccountId>, peer_id: Vec<u8>, resource_index: u64, config: ResourceConfig, rental_info: ResourceRentalInfo<BlockNumber>, price: u128, lock_price: u128, penalty_amount: u128, receive_amount: u128, start: BlockNumber, end: BlockNumber, calculation: BlockNumber,time:Duration) -> Self {
        RentalAgreement {
            index,
            provider,
            tenant_info,
            peer_id,
            resource_index,
            config,
            rental_info,
            price,
            lock_price,
            penalty_amount,
            receive_amount,
            start,
            end,
            calculation,
            time
        }
    }

    /// 执行协议
    pub fn execution(&mut self, block_number: &BlockNumber) -> bool {
        // 判断协议是否被惩罚
        if self.penalty_amount > 0 {
            return false
        }

        // 获得订单持续时间
        let duration = TryInto::<u128>::try_into(self.end.clone() - self.start.clone()).ok().unwrap();
        //如果在当前区块协议还没有结束
        if block_number < &self.end {
            // (现在的区块 - 上次上报的区块) / 协议共持续的区块 * 订单金额 = 这段时间获取的金额
            let this_block = block_number.clone() - self.calculation.clone();
            // 计算块数
            let this_block = TryInto::<u128>::try_into(this_block).ok().unwrap();
            // 计算这段时间获得得金额
            let price = (this_block * self.price.clone()) / duration;

            self.lock_price = self.lock_price - price;
            self.receive_amount = self.receive_amount + price;
            self.calculation = block_number.clone();
        } else {
            // 当前协议结束
            self.receive_amount += self.lock_price;
            self.lock_price = 0;
            self.calculation = self.end.clone();
        }

        true
    }

    /// 故障执行协议
    pub fn fault_excution(&mut self) -> u128{
        // 获取订单剩余金额
        let price = self.lock_price;

        // 将锁定金额全部转到惩罚金额
        self.penalty_amount += self.lock_price;
        // 锁定金额置0
        self.lock_price = 0;

        price
    }

    /// 取回金额
    pub fn withdraw(&mut self) -> u128{
        let price = self.receive_amount;
        self.receive_amount = 0;
        price
    }

    /// 取回惩罚金额
    pub fn withdraw_penalty(&mut self) -> u128 {
        let price = self.penalty_amount;
        self.penalty_amount = 0;
        price
    }

    /// 续费
    pub fn renew(&mut self,price:u128,duration:BlockNumber,resource_config:ComputingResource<BlockNumber,AccountId>) {
        // 协议价格增加
        self.price += price;
        self.lock_price += price;
        // 协议结束期限增加
        self.end += duration;
        // 更新协议资源快照
        self.rental_info = resource_config.rental_info;
        self.config = resource_config.config;
    }

    /// 判断协议是否完成
    pub fn is_finished(self) -> bool {
        if self.lock_price == 0 && self.penalty_amount == 0 && self.receive_amount == 0 {
            return true
        }

        false
    }
}

impl<AccountId> TenantInfo<AccountId> {
    pub fn new(account_id: AccountId, public_key: Bytes) -> Self {
        TenantInfo {
            account_id,
            public_key,
        }
    }
}

impl StakingAmount {
    /// 质押金额
    pub fn staking_amount(&mut self, price: u128) {
        self.amount += price;
        self.active_amount += price;
    }

    /// 锁定金额
    pub fn lock_amount(&mut self, price: u128) -> bool {
        if self.active_amount < price {
            return false;
        }
        self.active_amount -= price;
        self.lock_amount += price;
        true
    }

    /// 解锁金额
    pub fn unlock_amount(&mut self, price: u128) -> bool {
        if self.lock_amount < price {
            return false;
        }
        self.active_amount +=  price;
        self.lock_amount -=  price;

        true
    }

    /// 取回金额
    pub fn withdraw_amount(&mut self,price:u128) -> bool {
        if self.active_amount < price {
            return false;
        }

        self.amount -= price;
        self.active_amount -= price;
        true
    }

    /// 惩罚金额
    pub fn penalty_amount(&mut self, price:u128) {
        self.amount -= price;
        self.active_amount = self.active_amount + self.lock_amount - price;
        self.lock_amount = 0 ;
    }
}


pub trait OrderInterface {
    type AccountId;
    type BlockNumber: Parameter + AtLeast32BitUnsigned;
    /// 获取算力资源信息
    fn get_computing_resource_info(index: u64) -> Option<ComputingResource<Self::BlockNumber, Self::AccountId>> ;

    /// 更新资源接口
    fn update_computing_resource(index: u64, resource_info: ComputingResource<Self::BlockNumber,Self::AccountId>);
}
