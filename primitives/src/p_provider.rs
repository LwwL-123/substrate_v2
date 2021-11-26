use codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_debug_derive::RuntimeDebug;
use sp_std::vec::Vec;


/// 算力资源
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct ComputingResource<BlockNumber, AccountId> {
    /// 算力资源索引
    pub index: u64,
    /// 提供者账户
    pub account_id: AccountId,
    /// 计算资源链接ID
    pub peer_id: Vec<u8>,
    /// 资源配置
    pub config: ResourceConfig,
    /// 资源出租统计
    pub rental_statistics: ResourceRentalStatistics,
    /// 资源出租信息
    pub rental_info: ResourceRentalInfo<BlockNumber>,
    /// 资源租用状态
    pub status: ResourceStatus,

}

impl<BlockNumber, AccountId> ComputingResource<BlockNumber, AccountId> {
    pub fn new(index: u64,
               account_id: AccountId,
               peer_id: Vec<u8>,
               config: ResourceConfig,
               rental_statistics: ResourceRentalStatistics,
               rental_info: ResourceRentalInfo<BlockNumber>,
               status: ResourceStatus) -> Self {
        ComputingResource {
            index,
            account_id,
            peer_id,
            config,
            rental_statistics,
            rental_info,
            status,
        }
    }

    /// 更新单价
    pub fn update_rental_unit_price(&mut self, rent_unit_price: u128) {
        self.rental_info.set_rent_unit_price(rent_unit_price);
    }

    /// 更新状态
    pub fn update_status(&mut self, status: ResourceStatus) {
        self.status = status
    }
}


#[derive(Encode, Decode, RuntimeDebug, PartialEq, Eq, Copy, Clone)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum ResourceStatus {
    /// 使用中
    Inuse,
    /// 已锁定
    Locked,
    /// 未使用
    Unused,
}


/// 资源配置
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct ResourceConfig {
    pub cpu: u64,
    pub memory: u64,
    pub system: Vec<u8>,
    pub cpu_model: Vec<u8>,
}

impl ResourceConfig {
    pub fn new(cpu: u64, memory: u64, system: Vec<u8>, cpu_model: Vec<u8>) -> Self {
        Self {
            cpu,
            memory,
            system,
            cpu_model,
        }
    }
}


/// 资源统计信息
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct ResourceRentalStatistics {
    /// 租用个数
    pub rental_count: u32,
    /// 租用时长
    pub rental_duration: u32,
    /// 故障次数
    pub fault_count: u32,
    /// 故障时长
    pub fault_duration: u32,
}

impl ResourceRentalStatistics {
    pub fn new(
        rental_count: u32,
        rental_duration: u32,
        fault_count: u32,
        fault_duration: u32,
    ) -> Self {
        ResourceRentalStatistics {
            rental_count,
            rental_duration,
            fault_count,
            fault_duration,
        }
    }


    /// 增加租用次数
    pub fn add_rental_count(&mut self) {
        self.rental_count = self.rental_count + 1;
    }

    /// 增加租用时长
    pub fn add_rental_duration(&mut self, rental_duration: u32) {
        self.rental_duration = self.rental_duration + rental_duration;
    }

    /// 增加故障次数
    pub fn add_fault_count(&mut self) {
        self.fault_count = self.fault_count + 1;
    }

    /// 增加故障时长
    pub fn add_fault_duration(&mut self, fault_duration: u32) {
        self.fault_duration = self.fault_duration + fault_duration;
    }
}


/// 资源出租信息
#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct ResourceRentalInfo<BlockNumber> {
    /// 出租单价
    pub rent_unit_price: u128,
    /// 提供出租时长
    pub rent_duration: BlockNumber,
    /// 结束出租区块
    pub end_of_rent: BlockNumber,
}

impl<BlockNumber> ResourceRentalInfo<BlockNumber> {
    pub fn new(rent_unit_price: u128,
               rent_duration: BlockNumber,
               end_of_rent: BlockNumber,
    ) -> Self {
        ResourceRentalInfo {
            rent_unit_price,
            rent_duration,
            end_of_rent,
        }
    }

    /// 设置租用单价
    pub fn set_rent_unit_price(&mut self, rent_unit_price: u128) -> &mut ResourceRentalInfo<BlockNumber> {
        self.rent_unit_price = rent_unit_price;
        self
    }
}

pub trait ProviderInterface {
    type BlockNumber;
    type AccountId;

    /// 获取计算资源信息
    fn get_computing_resource_info(index: u64) -> ComputingResource<Self::BlockNumber, Self::AccountId>;

    /// 更新算力资源信息
    fn update_computing_resource
    (index: u64, resource: ComputingResource<Self::AccountId, Self::AccountId>);
}

