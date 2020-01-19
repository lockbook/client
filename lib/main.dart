import 'package:client/account_helper.dart';
import 'package:client/db_provider.dart';
import 'package:client/encryption_helper.dart';
import 'package:client/network_helper.dart';
import 'package:client/user_repository.dart';
import 'package:client/welcome.dart';
import 'package:flutter/cupertino.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:zefyr/zefyr.dart';

import 'lockbook.dart';
import 'new_lockbook.dart';

// Compile Time Constants for Dependency Injection
const String apiBase = "http://lockbook.app:8000";
const EncryptionHelper encryptionHelper = EncryptionHelper();
const DBProvider dbProvider = DBProvider();
const UserRepository userRepository = UserRepository(dbProvider);
const NetworkHelper networkHelper = NetworkHelper(apiBase, userRepository);
const AccountHelper accountHelper =
    AccountHelper(encryptionHelper, userRepository, networkHelper);

const welcome = Welcome(dbProvider);
const NewLockbook newLockbook = NewLockbook(accountHelper);

void main() {
  WidgetsFlutterBinding.ensureInitialized();
  SystemChrome.setSystemUIOverlayStyle(SystemUiOverlayStyle(
    systemNavigationBarColor: Monokai.Dark, // navigation bar color
    statusBarColor: Monokai.Dark, // status bar color
  ));
  userRepository.getUserInfo().then((result) => result
      .ifSuccess((info) => runApp(Lockbook(info)))
      .ifFailure((_) => runApp(welcome)));
}

theme() => ThemeData(
      brightness: Brightness.dark,
      appBarTheme: AppBarTheme(color: Monokai.Dark, elevation: 0),
      scaffoldBackgroundColor: Monokai.Dark,
      buttonColor: Monokai.Yellow,
      hintColor: Monokai.Yellow,
      inputDecorationTheme: InputDecorationTheme(
        labelStyle: TextStyle(color: Monokai.Yellow),
        focusColor: Monokai.Yellow,
        hoverColor: Monokai.Yellow,
        fillColor: Monokai.Yellow,
        enabledBorder: UnderlineInputBorder(
          borderSide: BorderSide(color: Monokai.Yellow),
        ),
        focusedBorder: UnderlineInputBorder(
          borderSide: BorderSide(color: Monokai.Yellow),
        ),
      ),
    );

zefyrTheme() => ZefyrThemeData(
      cursorColor: Monokai.Yellow,
      paragraphTheme: StyleTheme(
        textStyle: TextStyle(color: Monokai.White),
      ),
    );

class Monokai {
  static const Color White = const Color(0xFFFFFFFF);
  static const Color Dark = const Color(0xFF2C292D);
  static const Color Yellow = const Color(0xFFFFD866);
  static const Color Green = const Color(0xFFA9DC76);
  static const Color Red = const Color(0xFFFF6188);
  static const Color Blue = const Color(0xFF78DCE8);
  static const Color Purple = const Color(0xFFAB9DF2);
}
