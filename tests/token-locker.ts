import * as anchor from "@project-serum/anchor";
import { Program } from "@project-serum/anchor";
import { BN } from "bn.js";
import { TokenLocker } from "../target/types/token_locker";
import { PublicKey } from "@solana/web3.js";
import { ASSOCIATED_TOKEN_PROGRAM_ID, createMint, getOrCreateAssociatedTokenAccount, mintTo, TOKEN_PROGRAM_ID } from "@solana/spl-token";
// import * as common from "@project-serum/common";
import { NodeWallet } from "@project-serum/common";



describe("token-locker", () => {

  function sleep(ms: number) {
    return new Promise(resolve => setTimeout(resolve, ms))
  }

  // Configure the client to use the local cluster.

  const provider = anchor.AnchorProvider.env();
  
  anchor.setProvider(provider);
  const baseAccount = anchor.web3.Keypair.generate();
  const feeAccount = anchor.web3.Keypair.generate();
  const recipient = anchor.web3.Keypair.generate();
  //@ts-ignore
  const user:NodeWallet = provider.wallet;

  const DECIMALS = 9;
  const start = new BN(+new Date() / 1000 + 5);
  const end = new BN(+new Date() / 1000 + 15); 

  const program = anchor.workspace.TokenLocker as Program<TokenLocker>;
  let mint: PublicKey;
  let userToken;
  before(async() => {
    // mint
    mint = await createMint(
      provider.connection,
      user.payer,
      user.publicKey,
      user.publicKey,
      DECIMALS
    );
    // get user associated token account
    userToken = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      user.payer,
      mint,
      user.publicKey
    );
    // mint
    await mintTo(
      provider.connection,
      user.payer,
      mint,
      userToken.address,
      user.publicKey,
      10 * anchor.web3.LAMPORTS_PER_SOL
    );
    console.log("minting")
  });

  it("Is initialized!", async () => {
    // Add your test here.
    const tx = await program.methods
      .initialize(
        new BN(1),
        new BN(3 * anchor.web3.LAMPORTS_PER_SOL),
      )
      .accounts(
         {
            baseAccount: baseAccount.publicKey,
            owner: provider.wallet.publicKey,
            feeAccount: feeAccount.publicKey,
            systemProgram: anchor.web3.SystemProgram.programId,
          }
      )
      .signers([baseAccount])
      .rpc();
    console.log("Your transaction signature", tx);
  });

  it("create user stats", async () => {
    // const [userStatsPDA, _] = PublicKey
    //   .findProgramAddressSync(
    //     [
    //       anchor.utils.bytes.utf8.encode("user-stats"),
    //       provider.wallet.publicKey.toBuffer()
    //     ],
    //     program.programId
    //   );

    // await program.methods
    //   .createUserStats()
    //   .accounts({
    //     user: provider.wallet.publicKey,
    //     userStats: userStatsPDA,
    //     systemProgram: anchor.web3.SystemProgram.programId
    //   })
    //   .rpc();

  });

  it("Creating vesting", async () => {
    const [userStatsPDA, _] = PublicKey
      .findProgramAddressSync(
        [
          anchor.utils.bytes.utf8.encode("user-stats"),
          provider.wallet.publicKey.toBuffer()
        ],
        program.programId
      );
    const [vaultPDA, nonce] = PublicKey
      .findProgramAddressSync(
        [
          baseAccount.publicKey.toBuffer()
        ],
        program.programId
      );
    const recipientToken = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      user.payer,
      mint,
      recipient.publicKey
    );
    const feeToken = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      user.payer,
      mint,
      feeAccount.publicKey
    );
    const tx = await program.methods
      .createVesting(
        new BN(10 * anchor.web3.LAMPORTS_PER_SOL),
        start,
        end,
        true,
        nonce
      )
      .accounts({
        user: user.publicKey,
        userStats: userStatsPDA,
        userToken: userToken.address,
        recipient: recipient.publicKey,
        recipientToken: recipientToken.address,
        feeToken: feeToken.address,
        mint: mint,
        vault: vaultPDA,
        clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        baseAccount: baseAccount.publicKey,
        feeAccount: feeAccount.publicKey
      })
      .signers([])
      .rpc();
      console.log("Your transaction signature", tx);
  });

  it("unlock", async () => {
    await sleep(30000);
    const [userStatsPDA, _] = PublicKey
      .findProgramAddressSync(
        [
          anchor.utils.bytes.utf8.encode("user-stats"),
          provider.wallet.publicKey.toBuffer()
        ],
        program.programId
      );

    const account = await program.account.userStats.fetch(userStatsPDA);
    const vestList = account.vestList;
    const [vaultPDA, nonce] = PublicKey
      .findProgramAddressSync(
        [
          baseAccount.publicKey.toBuffer()
        ],
        program.programId
      );
    const recipientToken = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      user.payer,
      mint,
      recipient.publicKey
    );

    const tx = await program.methods
      .unlock(
        vestList.length - 1,
        new BN(10 * anchor.web3.LAMPORTS_PER_SOL),
      )
      .accounts({
        user: user.publicKey,
        userStats: userStatsPDA,
        recipientToken: recipientToken.address,
        mint: mint,
        vault: vaultPDA,
        clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
        tokenProgram: TOKEN_PROGRAM_ID,
        baseAccount: baseAccount.publicKey,
      })
      .signers([])
      .rpc();
      console.log("Your transaction signature", tx);
  });
});
