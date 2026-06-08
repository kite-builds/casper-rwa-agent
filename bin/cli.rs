//! odra-cli entrypoint for the CasperRWA-Agent `RwaVault` contract.
//!
//! Provides:
//!   * a deploy script that installs `RwaVault` on the configured network
//!     (livenet config read from `.env`: node address, chain name, secret key);
//!   * scenarios that exercise the full on-chain settlement flow used by the
//!     autonomous agent: `register` shareholders, `deposit` rent, `distribute`
//!     pro-rata, and `settle` (the three combined in one run).
//!
//! Run examples (after `cargo odra build` and a populated `.env`):
//!   cargo run --bin casper_rwa_agent_cli -- deploy --deploy-mode build
//!   cargo run --bin casper_rwa_agent_cli -- scenario settle \
//!       --holder1 account-hash-<..> --holder2 account-hash-<..> --rent 200
//!   cargo run --bin casper_rwa_agent_cli -- scenario distribute

use std::str::FromStr;

use casper_rwa_agent::rwa_vault::{RwaVault, RwaVaultInitArgs};
use odra::casper_types::U512;
use odra::host::{HostEnv, HostRef};
use odra::prelude::Address;
use odra::schema::casper_contract_schema::NamedCLType;
use odra_cli::{
    deploy::DeployScript,
    scenario::{Args, Error, Scenario, ScenarioMetadata},
    CommandArg, ContractProvider, DeployedContractsContainer, DeployerExt, OdraCli,
};

/// Gas budget for installing the vault contract.
const DEPLOY_GAS: u64 = 300_000_000_000;
/// Gas budget for a plain entry-point call.
const CALL_GAS: u64 = 5_000_000_000;
/// Gas budget for a payable call routed through the Odra proxy caller (heavier).
const PAYABLE_GAS: u64 = 25_000_000_000;
/// Gas budget for `distribute` (loops over shareholders + native transfers).
const DISTRIBUTE_GAS: u64 = 20_000_000_000;

/// Deploys `RwaVault` initialised for one tokenized asset.
pub struct VaultDeployScript;

impl DeployScript for VaultDeployScript {
    fn deploy(
        &self,
        env: &HostEnv,
        container: &mut DeployedContractsContainer,
    ) -> Result<(), odra_cli::deploy::Error> {
        let _vault = RwaVault::load_or_deploy(
            env,
            RwaVaultInitArgs {
                asset: String::from("Oslo duplex, Storgata 1 (tokenized RWA)"),
            },
            container,
            DEPLOY_GAS,
        )?;
        Ok(())
    }
}

fn parse_addr(s: &str) -> Result<Address, Error> {
    Address::from_str(s).map_err(|e| Error::OdraError {
        message: format!("invalid address '{s}': {e:?}"),
    })
}

/// Registers a single shareholder.
pub struct RegisterScenario;

impl Scenario for RegisterScenario {
    fn args(&self) -> Vec<CommandArg> {
        vec![
            CommandArg::new("holder", "Shareholder account-hash address", NamedCLType::String),
            CommandArg::new("units", "Share units to assign", NamedCLType::U64),
        ]
    }

    fn run(&self, env: &HostEnv, container: &DeployedContractsContainer, args: Args) -> Result<(), Error> {
        let mut vault = container.contract_ref::<RwaVault>(env)?;
        let holder = parse_addr(&args.get_single::<String>("holder")?)?;
        let units = args.get_single::<u64>("units")?;
        env.set_gas(CALL_GAS);
        vault.register_shareholder(holder, U512::from(units));
        odra_cli::log(format!("registered {holder:?} with {units} units"));
        Ok(())
    }
}

impl ScenarioMetadata for RegisterScenario {
    const NAME: &'static str = "register";
    const DESCRIPTION: &'static str = "Register a fractional shareholder";
}

/// Deposits rent into the undistributed pool.
pub struct DepositScenario;

impl Scenario for DepositScenario {
    fn args(&self) -> Vec<CommandArg> {
        vec![CommandArg::new("rent", "Rent to deposit, in CSPR", NamedCLType::U64)]
    }

    fn run(&self, env: &HostEnv, container: &DeployedContractsContainer, args: Args) -> Result<(), Error> {
        let mut vault = container.contract_ref::<RwaVault>(env)?;
        let rent_cspr = args.get_single::<u64>("rent")?;
        let motes = U512::from(rent_cspr) * U512::from(1_000_000_000u64);
        env.set_gas(PAYABLE_GAS);
        vault.with_tokens(motes).deposit_rent();
        odra_cli::log(format!("deposited {rent_cspr} CSPR ({motes} motes) of rent"));
        Ok(())
    }
}

impl ScenarioMetadata for DepositScenario {
    const NAME: &'static str = "deposit";
    const DESCRIPTION: &'static str = "Deposit rent (native CSPR) into the pool";
}

/// Distributes the rent pool pro-rata to all shareholders (the settlement tx).
pub struct DistributeScenario;

impl Scenario for DistributeScenario {
    fn run(&self, env: &HostEnv, container: &DeployedContractsContainer, _args: Args) -> Result<(), Error> {
        let mut vault = container.contract_ref::<RwaVault>(env)?;
        env.set_gas(DISTRIBUTE_GAS);
        vault.distribute();
        odra_cli::log(format!(
            "distributed; remaining pool {} motes, lifetime {} motes",
            vault.rent_pool(),
            vault.total_distributed()
        ));
        Ok(())
    }
}

impl ScenarioMetadata for DistributeScenario {
    const NAME: &'static str = "distribute";
    const DESCRIPTION: &'static str = "Distribute the rent pool pro-rata (settlement tx)";
}

/// Full settlement flow in one run: register two shareholders, deposit rent,
/// distribute. This is the human-readable mirror of what the agent does.
pub struct SettleScenario;

impl Scenario for SettleScenario {
    fn args(&self) -> Vec<CommandArg> {
        vec![
            CommandArg::new("holder1", "First shareholder account-hash", NamedCLType::String),
            CommandArg::new("holder2", "Second shareholder account-hash", NamedCLType::String),
            CommandArg::new("rent", "Rent to deposit, in CSPR", NamedCLType::U64),
        ]
    }

    fn run(&self, env: &HostEnv, container: &DeployedContractsContainer, args: Args) -> Result<(), Error> {
        let mut vault = container.contract_ref::<RwaVault>(env)?;
        let h1 = parse_addr(&args.get_single::<String>("holder1")?)?;
        let h2 = parse_addr(&args.get_single::<String>("holder2")?)?;
        let rent_cspr = args.get_single::<u64>("rent")?;
        let motes = U512::from(rent_cspr) * U512::from(1_000_000_000u64);

        if vault.shareholder_count() == 0 {
            env.set_gas(CALL_GAS);
            vault.register_shareholder(h1, U512::from(60u64));
            env.set_gas(CALL_GAS);
            vault.register_shareholder(h2, U512::from(40u64));
            odra_cli::log("registered holder1 (60) + holder2 (40)");
        } else {
            odra_cli::log(format!(
                "shareholders already registered ({})",
                vault.shareholder_count()
            ));
        }

        env.set_gas(PAYABLE_GAS);
        vault.with_tokens(motes).deposit_rent();
        odra_cli::log(format!("deposited {rent_cspr} CSPR rent"));

        env.set_gas(DISTRIBUTE_GAS);
        vault.distribute();
        odra_cli::log(format!(
            "settled: lifetime distributed {} motes, dust pool {} motes",
            vault.total_distributed(),
            vault.rent_pool()
        ));
        Ok(())
    }
}

impl ScenarioMetadata for SettleScenario {
    const NAME: &'static str = "settle";
    const DESCRIPTION: &'static str = "Register + deposit + distribute in one run";
}

/// CLI entrypoint.
pub fn main() {
    OdraCli::new()
        .about("CLI for the CasperRWA-Agent RwaVault contract")
        .deploy(VaultDeployScript)
        .contract::<RwaVault>()
        .scenario(RegisterScenario)
        .scenario(DepositScenario)
        .scenario(DistributeScenario)
        .scenario(SettleScenario)
        .build()
        .run();
}
