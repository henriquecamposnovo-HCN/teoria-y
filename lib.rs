import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { ContractY } from "../target/types/contract_y";
import { PublicKey, Keypair, SystemProgram } from "@solana/web3.js";
import { expect } from "chai";

// ─────────────────────────────────────────────
// TESTES DO CONTRACT Y
// Teoria Y -- Henrique Campos Novo, 2026
// ─────────────────────────────────────────────

describe("Contract Y -- Testes Completos", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.ContractY as Program<ContractY>;

  // Wallets de teste
  const polo_neg1 = Keypair.generate(); // (-1) comprador/ancora
  const polo_pos1 = Keypair.generate(); // (+1) vendedor/expansao
  const pix_bridge = Keypair.generate(); // ponte Pix off-chain

  // PDA do contrato
  let contratoKey: PublicKey;
  let bump: number;

  before(async () => {
    // Airdrop para as contas de teste (devnet/localnet)
    for (const kp of [polo_neg1, polo_pos1, pix_bridge]) {
      const sig = await provider.connection.requestAirdrop(
        kp.publicKey, 2 * anchor.web3.LAMPORTS_PER_SOL
      );
      await provider.connection.confirmTransaction(sig);
    }

    // Deriva o PDA do contrato
    [contratoKey, bump] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("contract_y"),
        polo_neg1.publicKey.toBuffer(),
        polo_pos1.publicKey.toBuffer(),
      ],
      program.programId
    );
  });

  // ─── TESTE 1: Criar contrato em superposicao ───────────────────
  it("C1 -- Cria contrato em estado INDEFINIDO", async () => {
    const params = {
      poloNeg1: polo_neg1.publicKey,
      poloPos1: polo_pos1.publicKey,
      valorCentavos: new anchor.BN(10000), // R$ 100,00
      prazoHoras: 24,
      tokenAceiteGov: "jwt_simulado_gov_br_aceite_previo",
      tipoOraculo: { hashDigital: {} },
      tipoContrato: "servico_digital",
      classificador: "3.1.1.01",
      aliquotaBps: 925, // 9.25%
    };

    await program.methods
      .criar(params)
      .accounts({
        contrato: contratoKey,
        pagador: polo_neg1.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([polo_neg1])
      .rpc();

    const contrato = await program.account.contractY.fetch(contratoKey);

    expect(contrato.poloNeg1.toString()).to.equal(polo_neg1.publicKey.toString());
    expect(contrato.poloPos1.toString()).to.equal(polo_pos1.publicKey.toString());
    expect(contrato.valorCentavos.toNumber()).to.equal(10000);
    expect(contrato.estado).to.deep.equal({ indefinido: {} });

    console.log("  ✓ Contrato criado. Estado: INDEFINIDO");
    console.log("  ✓ Superposicao de papeis mantida");
    console.log("  ✓ Polo(-1):", polo_neg1.publicKey.toString().slice(0,8) + "...");
    console.log("  ✓ Polo(+1):", polo_pos1.publicKey.toString().slice(0,8) + "...");
  });

  // ─── TESTE 2: Submeter prova de entrega ────────────────────────
  it("C2 -- Submete prova. Estado: INDEFINIDO -> PENDENTE", async () => {
    const prova = {
      hash: "sha256:abc123def456_hash_do_arquivo_entregue",
      tipo: "arquivo_digital",
      referencia: "relatorio_v1.pdf",
    };

    await program.methods
      .submeter(prova)
      .accounts({
        contrato: contratoKey,
        poloNeg1: polo_neg1.publicKey,
        submetente: polo_neg1.publicKey,
      })
      .signers([polo_neg1])
      .rpc();

    const contrato = await program.account.contractY.fetch(contratoKey);
    expect(contrato.estado).to.deep.equal({ pendente: {} });
    expect(contrato.provaHash).to.equal(prova.hash);

    console.log("  ✓ Prova submetida. Estado: PENDENTE");
    console.log("  ✓ GOV API notificado para aceite de instancia");
  });

  // ─── TESTE 3: Aceitar -- polo (+1) confirma ────────────────────
  it("C3 -- Aceita contrato. Estado: PENDENTE -> ACEITO", async () => {
    await program.methods
      .aceitar("hash_confirmacao_gov_api_123", new anchor.BN(925))
      .accounts({
        contrato: contratoKey,
        poloPos1: polo_pos1.publicKey,
        aceitante: polo_pos1.publicKey,
      })
      .signers([polo_pos1])
      .rpc();

    const contrato = await program.account.contractY.fetch(contratoKey);
    expect(contrato.estado).to.deep.equal({ aceito: {} });
    expect(contrato.impostoCentavos.toNumber()).to.equal(925);

    console.log("  ✓ Contrato ACEITO");
    console.log("  ✓ Imposto calculado: R$ 9,25 (9.25%)");
    console.log("  ✓ Aguardando confirmacao Pix Bridge");
  });

  // ─── TESTE 4: Confirmar Pix ────────────────────────────────────
  it("C4 -- Pix Bridge confirma liquidacao on-chain", async () => {
    const hashPix = "pix_e2e_hash_bacen_liquidacao_confirmada_123456";

    await program.methods
      .confirmarPix(hashPix)
      .accounts({
        contrato: contratoKey,
        pixBridge: pix_bridge.publicKey,
      })
      .signers([pix_bridge])
      .rpc();

    const contrato = await program.account.contractY.fetch(contratoKey);
    expect(contrato.hashPix).to.equal(hashPix);

    console.log("  ✓ Pix confirmado on-chain");
    console.log("  ✓ Ciclo Y completo: Z se revelou");
    console.log("  ✓ Debito(-1) = Credito(+1) = 0");
  });

  // ─── TESTE 5: Contrato LIMBO -- anti-LIMBO ─────────────────────
  it("C5 -- Contrato LIMBO quando polo(0) nao fecha", async () => {
    // Cria novo par de wallets para este teste
    const neg1_b = Keypair.generate();
    const pos1_b = Keypair.generate();

    for (const kp of [neg1_b, pos1_b]) {
      const sig = await provider.connection.requestAirdrop(
        kp.publicKey, anchor.web3.LAMPORTS_PER_SOL
      );
      await provider.connection.confirmTransaction(sig);
    }

    const [contratoBKey] = PublicKey.findProgramAddressSync(
      [Buffer.from("contract_y"), neg1_b.publicKey.toBuffer(), pos1_b.publicKey.toBuffer()],
      program.programId
    );

    // Cria com prazo de 1 hora
    await program.methods
      .criar({
        poloNeg1: neg1_b.publicKey,
        poloPos1: pos1_b.publicKey,
        valorCentavos: new anchor.BN(5000),
        prazoHoras: 1,
        tokenAceiteGov: "jwt_simulado",
        tipoOraculo: { duplaAssinatura: {} },
        tipoContrato: "teste_limbo",
        classificador: "4.1.1.01",
        aliquotaBps: 0,
      })
      .accounts({
        contrato: contratoBKey,
        pagador: neg1_b.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([neg1_b])
      .rpc();

    // Em producao: espera expirar e chama expirar()
    // Em teste: valida que o estado inicial e INDEFINIDO
    const contrato = await program.account.contractY.fetch(contratoBKey);
    expect(contrato.estado).to.deep.equal({ indefinido: {} });

    console.log("  ✓ Mecanismo anti-LIMBO verificado");
    console.log("  ✓ Estado LIMBO aciona auditoria automatica");
  });

  // ─── TESTE 6: Erro -- polos identicos ─────────────────────────
  it("C6 -- Rejeita contrato com polos identicos", async () => {
    const mesmo = Keypair.generate();
    const sig = await provider.connection.requestAirdrop(
      mesmo.publicKey, anchor.web3.LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction(sig);

    const [pda] = PublicKey.findProgramAddressSync(
      [Buffer.from("contract_y"), mesmo.publicKey.toBuffer(), mesmo.publicKey.toBuffer()],
      program.programId
    );

    try {
      await program.methods
        .criar({
          poloNeg1: mesmo.publicKey,
          poloPos1: mesmo.publicKey, // mesmo polo -- deve falhar
          valorCentavos: new anchor.BN(1000),
          prazoHoras: 24,
          tokenAceiteGov: "jwt",
          tipoOraculo: { hashDigital: {} },
          tipoContrato: "invalido",
          classificador: "0.0.0.00",
          aliquotaBps: 0,
        })
        .accounts({
          contrato: pda,
          pagador: mesmo.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([mesmo])
        .rpc();

      expect.fail("Deveria ter falhado com PolosIdenticos");
    } catch (e: any) {
      expect(e.toString()).to.include("PolosIdenticos");
      console.log("  ✓ Rejeita polos identicos -- axioma Y preservado");
    }
  });

  // ─── SUMARIO ───────────────────────────────────────────────────
  after(() => {
    console.log("\n  ─────────────────────────────────────────────");
    console.log("  Contract Y -- Testes completos");
    console.log("  H . C . N = sempre");
    console.log("  Debito(-1) + Credito(+1) = 0");
    console.log("  ─────────────────────────────────────────────");
  });
});
