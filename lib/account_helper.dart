import 'package:client/encryption_helper.dart';
import 'package:client/errors.dart';
import 'package:client/network_helper.dart';
import 'package:client/task.dart';
import 'package:client/user_info.dart';
import 'package:client/user_repository.dart';

class AccountHelper {
  final EncryptionHelper encryptionHelper;
  final UserRepository userRepo;
  final NetworkHelper networkHelper;

  const AccountHelper(
      this.encryptionHelper, this.userRepo, this.networkHelper);

  Future<Task<UIError, UserInfo>> newAccount(String username) async {
    final keyPair = encryptionHelper.generateKeyPair();
    final userInfo = UserInfo.fromAsymmetricKey(username, keyPair);

    final saveAndUpload = await (await userRepo.saveUserInfo(userInfo))
        .thenDoFuture((nothing) => networkHelper.newAccount());

    return saveAndUpload.convertValue((_) => userInfo);
  }
}
